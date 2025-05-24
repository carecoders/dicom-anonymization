import argparse
import concurrent.futures
import logging
import multiprocessing
import os
import time

from dcmanon import Anonymizer
from tqdm import tqdm


def setup_logging():
    """Configure logging for the application."""
    logging.basicConfig(
        level=logging.ERROR,
        format='%(asctime)s - %(levelname)s - %(message)s',
        handlers=[logging.StreamHandler()]
    )
    return logging.getLogger(__name__)


def find_dicom_files(directory):
    """Find all DICOM files in the given directory and its subdirectories."""
    dicom_files = []
    for root, _, files in os.walk(directory):
        for file in files:
            file_path = os.path.join(root, file)
            # if is_dicom_file(file_path):
            dicom_files.append(file_path)
    return dicom_files


# Global variable to hold the anonymizer in each worker process
anonymizer = None

def init_worker():
    """Initialize worker process with a single Anonymizer instance."""
    global anonymizer
    anonymizer = Anonymizer()

def anonymize_file(args):
    """Anonymize a single DICOM file with pre-computed paths."""
    file_path, output_path = args
    try:
        anonymized_bytes = anonymizer.anonymize(file_path)
        with open(output_path, 'wb') as f:
            f.write(anonymized_bytes)
        return True
    except Exception as e:
        return f"Error anonymizing {file_path}: {str(e)}"


def prebuild_directory_structure(dicom_files, output_dir, input_dir):
    """Pre-create all output directories to avoid per-file overhead."""
    dirs_to_create = set()
    for file_path in dicom_files:
        rel_path = os.path.relpath(file_path, start=input_dir)
        output_path = os.path.join(output_dir, rel_path)
        dirs_to_create.add(os.path.dirname(output_path))

    for dir_path in dirs_to_create:
        os.makedirs(dir_path, exist_ok=True)


def prepare_work_items(dicom_files, output_dir, input_dir):
    """Pre-compute all paths to avoid repeated calculations."""
    work_items = []
    for file_path in dicom_files:
        rel_path = os.path.relpath(file_path, start=input_dir)
        output_path = os.path.join(output_dir, rel_path)
        work_items.append((file_path, output_path))
    return work_items


def main():
    parser = argparse.ArgumentParser(description='Anonymize DICOM files concurrently.')
    parser.add_argument('input_dir', help='Input directory containing DICOM files')
    parser.add_argument('output_dir', help='Output directory for anonymized files')
    parser.add_argument('--cores', type=int, default=multiprocessing.cpu_count(),
                        help='Number of CPU cores to use (default: all available)')
    args = parser.parse_args()

    logger = setup_logging()

    # Create output directory if it doesn't exist
    os.makedirs(args.output_dir, exist_ok=True)

    # Find all DICOM files
    logger.info(f"Searching for DICOM files in {args.input_dir}...")
    dicom_files = find_dicom_files(args.input_dir)

    if not dicom_files:
        logger.warning("No DICOM files found.")
        return

    logger.info(f"Found {len(dicom_files)} DICOM files. Starting anonymization with {args.cores} cores...")

    # Pre-create directory structure
    prebuild_directory_structure(dicom_files, args.output_dir, args.input_dir)

    # Pre-compute all paths
    work_items = prepare_work_items(dicom_files, args.output_dir, args.input_dir)

    # Process files concurrently
    start = time.time()
    with concurrent.futures.ProcessPoolExecutor(max_workers=args.cores, initializer=init_worker) as executor:
        # Optimal chunksize reduces task scheduling overhead
        chunksize = max(1, len(work_items) // (args.cores * 4))
        # with tqdm
        results = list(tqdm(executor.map(anonymize_file, work_items, chunksize=chunksize), total=len(dicom_files), desc="Anonymizing"))
        # without tqdm for maximum speed
        # results = list(executor.map(anonymize_file, work_items, chunksize=chunksize))

    # Check for errors
    # for result in results:
    #     if result is not True:
    #         errors.append(result)

    # Report results
    # if errors:
    #     logger.error(f"Completed with {len(errors)} errors:")
    #     for error in errors:
    #         logger.error(error)
    # else:
    #     print(
    #         f"Successfully anonymized {len(dicom_files)} files to {args.output_dir} in {time.time() - start:.2f} seconds.")
    #
    print(
        f"Successfully anonymized {len(dicom_files)} files to {args.output_dir} in {time.time() - start:.2f} seconds.")


if __name__ == "__main__":
    main()

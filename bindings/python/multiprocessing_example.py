import argparse
import multiprocessing
import os
import time

from dcmanon import Anonymizer


def find_dicom_files(directory):
    """Find all DICOM files in the given directory and its subdirectories."""
    dicom_files = []
    for root, _, files in os.walk(directory):
        for file in files:
            file_path = os.path.join(root, file)
            dicom_files.append(file_path)
    return dicom_files


def init_worker():
    """Initialize worker process with a single Anonymizer instance."""
    global anonymizer
    anonymizer = Anonymizer()


def anonymize_file(args):
    """Anonymize a single DICOM file."""
    file_path, output_path = args

    try:
        anonymized_bytes = anonymizer.anonymize(file_path)

        # Ensure output directory exists
        # os.makedirs(os.path.dirname(output_path), exist_ok=True)

        with open(output_path, 'wb') as f:
            f.write(anonymized_bytes)
        return True
    except Exception as e:
        return f"Error anonymizing {file_path}: {str(e)}"


def main():
    parser = argparse.ArgumentParser(description='Anonymize DICOM files using multiprocessing.')
    parser.add_argument('input_dir', help='Input directory containing DICOM files')
    parser.add_argument('output_dir', help='Output directory for anonymized files')
    parser.add_argument('--cores', type=int, default=multiprocessing.cpu_count(),
                        help='Number of CPU cores to use (default: all available)')
    args = parser.parse_args()

    # Create output directory if it doesn't exist
    os.makedirs(args.output_dir, exist_ok=True)

    # Find all DICOM files
    print(f"Searching for DICOM files in {args.input_dir}...")
    dicom_files = find_dicom_files(args.input_dir)

    if not dicom_files:
        print("No DICOM files found.")
        return

    print(f"Found {len(dicom_files)} DICOM files. Starting multiprocessing anonymization with {args.cores} cores...")

    # Prepare work items with pre-computed paths
    work_items = []
    for file_path in dicom_files:
        rel_path = os.path.relpath(file_path, start=args.input_dir)
        output_path = os.path.join(args.output_dir, rel_path)
        work_items.append((file_path, output_path))

    start = time.time()

    # Create process pool and process files
    with multiprocessing.Pool(processes=args.cores, initializer=init_worker) as pool:
        results = pool.map(anonymize_file, work_items)

    elapsed = time.time() - start

    # Check for errors
    errors = [result for result in results if result is not True]

    # Report results
    if errors:
        print(f"Completed with {len(errors)} errors in {elapsed:.2f} seconds.")
        for error in errors:
            print(f"  {error}")
    else:
        print(f"Successfully anonymized {len(dicom_files)} files to {args.output_dir} in {elapsed:.2f} seconds.")
        print(f"Rate: {len(dicom_files) / elapsed:.1f} files/second")


if __name__ == "__main__":
    main()

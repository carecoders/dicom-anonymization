import argparse
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


def main():
    parser = argparse.ArgumentParser(description='Anonymize DICOM files sequentially.')
    parser.add_argument('input_dir', help='Input directory containing DICOM files')
    parser.add_argument('output_dir', help='Output directory for anonymized files')
    args = parser.parse_args()

    # Create output directory if it doesn't exist
    os.makedirs(args.output_dir, exist_ok=True)

    # Initialize the anonymizer
    anonymizer = Anonymizer()

    # Find all DICOM files
    print(f"Searching for DICOM files in {args.input_dir}...")
    dicom_files = find_dicom_files(args.input_dir)

    if not dicom_files:
        print("No DICOM files found.")
        return

    output_paths = []
    for file_path in dicom_files:
        rel_path = os.path.relpath(file_path, start=args.input_dir)
        output_path = os.path.join(args.output_dir, rel_path)
        output_paths.append(output_path)

    print(f"Found {len(dicom_files)} DICOM files. Starting sequential anonymization...")

    errors = []

    start = time.time()
    for i, file_path in enumerate(dicom_files):
        try:
            # Create relative path to maintain directory structure
            # rel_path = os.path.relpath(file_path, start=args.input_dir)
            # output_path = os.path.join(args.output_dir, rel_path)

            # Ensure output directory exists
            # os.makedirs(os.path.dirname(output_path), exist_ok=True)

            # Anonymize the file
            anonymized_bytes = anonymizer.anonymize(file_path)

            # Write the anonymized bytes to the output file
            with open(output_paths[i], 'wb') as f:
                f.write(anonymized_bytes)

            # Print progress every 50 files
            # if (i + 1) % 50 == 0:
            #     print(f"Processed {i + 1}/{len(dicom_files)} files...")

        except Exception as e:
            error_msg = f"Error anonymizing {file_path}: {str(e)}"
            errors.append(error_msg)
            print(error_msg)

    elapsed = time.time() - start

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

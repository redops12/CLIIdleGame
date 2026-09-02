#!/usr/bin/python3
import argparse

def normalize(file):
    with open(file, 'r') as f:
        lines = f.readlines()
    normalized_lines = []
    for line in lines:
        print(line)

        # to lowercase
        line = line.lower()
        # remove punctuation
        line = ''.join(char if char.isalpha() or char.isspace() else " " for char in line )

        line = line.strip()
        line = line.replace('\t', ' ')
        line = ' '.join(line.split())

        print(line)
        normalized_lines.append(line)

    joined_lines = ' '.join(normalized_lines)
    # split into 40 character chunks respecting word boundaries
    chunks = []
    while len(joined_lines) > 0:
        if len(joined_lines) <= 40:
            chunks.append(joined_lines)
            break
        else:
            # find the last space within the first 40 characters
            split_index = joined_lines.rfind(' ', 0, 40)
            if split_index == -1:
                split_index = 40  # no space found, split at 40
            chunks.append(joined_lines[:split_index])
            joined_lines = joined_lines[split_index:].lstrip()  # remove leading spaces

    normalized_lines = [chunk for chunk in chunks if chunk]  # remove empty chunks

    with open(file, 'w') as f:
        f.write('\n'.join(normalized_lines))

argument_parser = argparse.ArgumentParser(description='Normalize a text file.')
argument_parser.add_argument('file', type=str, help='The path to the text file to normalize.')
args = argument_parser.parse_args()

normalize(args.file)

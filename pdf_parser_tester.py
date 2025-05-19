#!/usr/bin/env python3
import os
import re
from datetime import datetime
import sys
import csv




if __name__ == "__main__":

    out_text=""

    with open('SSPHP Documentation/cis_benchmark_v8_doc_aks.csv') as csv_file:
        csv_reader = csv.reader(csv_file, skipinitialspace=True)
        line_count = 0
        for row in csv_reader:
            if line_count == 0:
                print(f'Column names are {", ".join(row)}')
                line_count += 1
            else:
                #print(f'\t{row}')
                #print("==========================================================================================")
                #print(f'Use Case : {row[0]}\n')
                #print(f'Title : {row[5]}\n')
                #print(f'Description : {row[8]}\n')
                #print(f'Rationale : {row[9]}\n')
                #print(f'Impact : {row[10]}\n')
                #print("\n")

                out_text = out_text + "\n==========================================================================================\n"
                out_text = out_text + f'Use Case : {row[0]}\n\n'
                out_text = out_text + f'Title : {row[5]}\n\n'
                out_text = out_text + f'Description : {row[8]}\n\n'
                out_text = out_text + f'Rationale : {row[9]}\n\n'
                out_text = out_text + f'Impact : {row[10]}\n\n'

                line_count += 1
                
        
        #print("==========================================================================================")
    out_text = out_text + "\n==========================================================================================\n"
    print(out_text)

    with open("/Users/ianpearl/Downloads/AKS Benchmark Controls Test Output.txt", "w") as f:
        f.write(out_text)

    print(f'Processed {line_count} lines.')
    
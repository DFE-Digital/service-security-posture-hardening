#!/usr/bin/env python3
import os
import re
from datetime import datetime
import sys
import csv


def main_list():

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



def main_dict():

    with open('SSPHP Documentation/cis_benchmark_v8_doc_aks.csv', newline='') as csv_file:
        csv_reader = csv.DictReader(csv_file, skipinitialspace=True)
        for row in csv_reader:
            print(f"Control : {row['ssphp.cis_benchmark.control.number']}\n")
            print(f"Use Case : {row['ssphp.use_case.title']}\n")
            print(f"Title : {row['ssphp.cis_benchmark.control.title']}\n")
            print(f"Description : {row['ssphp.cis_benchmark.control.description']}\n")
            print(f"Rationale : {row['ssphp.cis_benchmark.control.rationale']}\n")
            print(f"Impact : {row['ssphp.cis_benchmark.control.impact']}\n")

'''ssphp.cis_benchmark.control.number
ssphp.use_case.foundational_system
ssphp.use_case.id
ssphp.use_case.savedsearch
ssphp.use_case.title
ssphp.cis_benchmark.control.title
ssphp.cis_benchmark.control.profile_applicability
ssphp.cis_benchmark.control.level
ssphp.cis_benchmark.control.description
ssphp.cis_benchmark.control.rationale
ssphp.cis_benchmark.control.impact
ssphp.cis_benchmark.control.group
ssphp.cis_benchmark.controls.v8
ssphp.cis_benchmark.controls.ig1
ssphp.cis_benchmark.controls.ig2
ssphp.cis_benchmark.controls.ig3
ssphp.use_case.framework.ig_1
ssphp.use_case.framework.ig_2
ssphp.use_case.framework.ig_3
ssphp.cis_benchmark.document.name
ssphp.cis_benchmark.document.version
ssphp.cis_benchmark.document.date
ssphp.cis_benchmark.version
ssphp.metadata.last_updated_by
ssphp.metadata.last_updated_date'''



if __name__ == "__main__":

    #main_list()
    main_dict()

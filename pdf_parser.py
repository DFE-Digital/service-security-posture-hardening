#!/usr/bin/env python3
import os
import re
from pypdf import PdfReader
from datetime import datetime
import sys



foundational_system = "AKS"
sections = []

#

def Parse_IG(ig_text):
    cis_controls=list()
    cis_controls_ig1=list()
    cis_controls_ig2=list()
    cis_controls_ig3=list()
    ig1=list()
    ig2=list()
    ig3=list()

    #ig_text = re.sub(r"^\sv8","v8",ig_text)
    #ig_text = re.sub(r"^\sv7","v7",ig_text)
    #ig_text = ig_text.replace(" v8","\nv8")
    #ig_text = ig_text.replace(" v7","\nv7")
    #ig_text = ig_text.replace("   ","|")

    if foundational_system == "GITHUB":
        ig_text = re.sub(r"([a-zA-Z0-9]+) {2,99}([a-zA-Z0-9]+)",r"\1|\2",ig_text)
    else:
        ig_text = ig_text.replace("\n ","|")
        ig_text = re.sub(r" *\|","|",ig_text)

    ig_text = ig_text.replace(" v7","~~~v7")
    ig_text = ig_text.replace(" v8","~~~v8")
    ig_text = ig_text.replace("\nv7","~~~v7")
    ig_text = ig_text.replace("\nv8","~~~v8")
    ig_text = ig_text.replace("\n","")
    ig_text = ig_text.replace("~~~v7","\nv7")
    ig_text = ig_text.replace("~~~v8","\nv8")

    for line in ig_text.splitlines():

        ctl = re.match(r"\s*(?P<s_v>v[7|8])\s(?P<s_stl>\d+\.\d+)\s(?P<s_txt>[^|]*)\|",line)
        if ctl:
            if ctl["s_v"]=="v8":
                ig_count = line.count("\u25cf")
                cis_controls.append(f'{ctl["s_stl"]} {ctl["s_txt"].replace("  "," ")}')

                if ig_count>0:
                    cis_controls_ig3.append(f'{ctl["s_stl"]}')
                    ig3.append("True")

                if ig_count>1:
                    cis_controls_ig2.append(f'{ctl["s_stl"]}')
                    ig2.append("True")

                if ig_count>2:
                    cis_controls_ig1.append(f'{ctl["s_stl"]}')
                    ig1.append("True")


    cis_controls_txt = ", ".join(cis_controls)
    cis_controls_ig1_txt = ", ".join(cis_controls_ig1)
    cis_controls_ig2_txt = ", ".join(cis_controls_ig2)
    cis_controls_ig3_txt = ", ".join(cis_controls_ig3)

    if not cis_controls_txt:
        cis_controls_txt = "-"

    if not cis_controls_ig1_txt:
        cis_controls_ig1_txt = "-"

    if not cis_controls_ig2_txt:
        cis_controls_ig2_txt = "-"

    if not cis_controls_ig3_txt:
        cis_controls_ig3_txt = "-"


    ig1_txt = ", ".join(ig1)
    ig2_txt = ", ".join(ig2)
    ig3_txt = ", ".join(ig3)

    if ig1_txt: 
        ig1_txt = "TRUE" 
    else: 
        ig1_txt = "FALSE"

    if ig2_txt: 
        ig2_txt = "TRUE" 
    else: 
        ig2_txt = "FALSE"

    if ig3_txt: 
        ig3_txt = "TRUE" 
    else: 
        ig3_txt = "FALSE"


    ig_parse_out = {
        "cis_controls": cis_controls_txt,
        "cis_controls_ig1": cis_controls_ig1_txt,
        "cis_controls_ig2": cis_controls_ig2_txt,
        "cis_controls_ig3": cis_controls_ig3_txt,
        "ig1": ig1_txt,
        "ig2": ig2_txt,
        "ig3": ig3_txt
    }

    return ig_parse_out



def Parse_Control(control_text):
    #print(control_text)

    ig_parse_out = {}

    # Eliminate all pdf translation funnies that put spaces in the middle of key words
    control_text = re.sub(r"P\s*r\s*o\s*f\s*i\s*l\s*e\s*A\s*p\s*p\s*l\s*i\s*c\s*a\s*b\s*i\s*l\s*i\s*t\s*y\s*:","Profile Applicability:",control_text)
    control_text = re.sub(r"D\s*e\s*s\s*c\s*r\s*i\s*p\s*t\s*i\s*o\s*n\s*:","Description:",control_text)
    control_text = re.sub(r"R\s*a\s*t\s*i\s*o\s*n\s*a\s*l\s*e\s*:","Rationale:",control_text)
    control_text = re.sub(r"I\s*m\s*p\s*a\s*c\s*t\s*:","Impact:",control_text)

    # Extract the fields from the Control Text
    match_1 = re.match(r"\s*Page\s+\d+\s+(?P<use_case_id>\d\d?\.\d\d?\.?\d?\d?)\s(?P<title>[\s\S]*?)P\s*r\s*o\s*f\s*i\s*l\s*e\s+A\s*p\s*p\s*l\s*i\s*c\s*a\s*b\s*i\s*l\s*i\s*t\s*y\s*:", control_text)
    match_2 = re.search(r"P\s*r\s*o\s*f\s*i\s*l\s*e\s+A\s*p\s*p\s*l\s*i\s*c\s*a\s*b\s*i\s*l\s*i\s*t\s*y\s*:(?P<profile_applicability>[\s\S]*?)\sD\s*e\s*s\s*c\s*r\s*i\s*p\s*t\s*i\s*o\s*n\s*:", control_text)
    match_3 = re.search(r"D\s*e\s*s\s*c\s*r\s*i\s*p\s*t\s*i\s*o\s*n\s*:(?P<description>[\s\S]*?)\sR\s*a\s*t\s*i\s*o\s*n\s*a\s*l\s*e\s*:", control_text)
    match_4 = re.search(r"R\s*a\s*t\s*i\s*o\s*n\s*a\s*l\s*e\s*:(?P<rationale>[\s\S]*?)\s(I\s*m\s*p\s*a\s*c\s*t\s*:|A\s*u\s*d\s*i\s*t\s*:)", control_text)
    match_5 = re.search(r"I\s*m\s*p\s*a\s*c\s*t\s*:(?P<impact>[\s\S]*?)\sA\s*u\s*d\s*i\s*t\s*:", control_text)
    match_6 = re.search(r"C\s*I\s*S\s*C\s*o\s*n\s*t\s*r\s*o\s*l\s*s\s*:\s*C\s*o\s*n\s*t\s*r\s*o\s*l\s*s\s*V\s*e\s*r\s*s\s*i\s*o\s*n\s*\s*C\s*o\s*n\s*t\s*r\s*o\s*l\s*I\s*G\s*1\s*I\s*G\s*2\s*I\s*G\s*3(?P<ig_data>[\s\S]*)$",control_text)


    # Clean up the data in the fields
    if match_1:
        id = match_1["use_case_id"]
        title = match_1["title"]

        title = re.sub(r"\(L(1|2)\)\s","",title)
        title = re.sub(r"\s\((A\s*u\s*t\s*o\s*m\s*a\s*t\s*e\s*d|M\s*a\s*n\s*u\s*a\s*l)\)","",title)
        title = re.sub(r"\u2022","",title)
        title = re.sub(r"(‘|’)","\'",title)
        title = title.replace("\n","")
        title = title.replace("  "," ")
        title = title.replace(" -","-")
        title = title.strip()

    if match_2:
        profile_applicability = match_2["profile_applicability"]
        profile_applicability = re.sub(r"\u2022","",profile_applicability)
        profile_applicability = re.sub(r"(‘|’)","\'",profile_applicability)
        profile_applicability = re.sub(r"Page\s\d*$","",profile_applicability)
        profile_applicability = re.sub(r"\n","",profile_applicability.strip())
        profile_applicability = profile_applicability.replace("  "," ")
        profile_applicability = profile_applicability.replace(" -","-")
    else:
        profile_applicability = ""

    if match_3:
        description = match_3["description"]
        description = re.sub(r"\u2022"," -",description)
        description = re.sub(r"(‘|’)","\'",description)
        description = re.sub(r"Page\s\d*$","",description)
        description = re.sub(r"\n","",description.strip())
        description = description.replace("\"","\'")
        description = description.replace("  "," ")
        description = description.replace(" -","-")
    else:
        description = ""

    if match_4:
        rationale = match_4["rationale"]
        rationale = re.sub(r"\u2022"," -",rationale)
        rationale = re.sub(r"(‘|’)","\'",rationale)
        rationale = re.sub(r"Page\s\d*$","",rationale)
        rationale = re.sub(r"\n","",rationale.strip())
        rationale = rationale.replace("\"","\'")
        rationale = rationale.replace("  "," ")
        rationale = rationale.replace(" -","-")
    else:
        rationale = ""
 
    if match_5:
        impact = match_5["impact"]
        impact = re.sub(r"\u2022"," -",impact)
        impact = re.sub(r"(‘|’)","\'",impact)
        impact = re.sub(r"Page\s\d*$","",impact)
        impact = re.sub(r"\n","",impact.strip())
        impact = impact.replace("\"","\'")
        impact = impact.replace("  "," ")
        impact = impact.replace(" -","-")
    else:
        impact = ""


    # Section Header
    id_section = re.match(r"(?P<match_sect>\d+)",id)

    if id_section:
        section = id_section["match_sect"]

        section_header = sections[int(section) - 1]["name"]
    else:
        section_header = "-"   


    # Parse out the IG control sections
    if match_6:
        ig_data = match_6["ig_data"]
    else:
        ig_data = ""
    ig_parse_out = Parse_IG(ig_data)

    # Build the Use Case Title, ID, SavedSearch
    use_case_group = re.match(r"(?P<ucg>[^\.]+).*",id)
    ucg = "00" + use_case_group["ucg"]
    ucg = ucg[len(ucg)-3:]
    use_case_title = f'{foundational_system} {ucg} [CIS {id}]'
    use_case_id = f'{foundational_system.lower()}_{ucg}_cis_{id.replace(".","-")}'
    use_case_savedsearch = f'ssphp_use_case_{use_case_id}'


    # Level
    level_match = re.match(r"(E(3|5))? *Level (?P<level>\d)",profile_applicability)
    if level_match:
        level = "L"+str(level_match["level"])
    else:
        level = "-"

    # Build the Ouput Dict ready to return
    control_data = {
        "id": id,
        "use_case_title": use_case_title,
        "title": title,
        "use_case_id": use_case_id,
        "use_case_savedsearch": use_case_savedsearch,
        "profile_applicability": profile_applicability,
        "level": level,
        "description": description,
        "rationale": rationale,
        "impact": impact,
        "section_header": section_header,
        "cis_controls": ig_parse_out["cis_controls"],
        "cis_controls_ig1": ig_parse_out["cis_controls_ig1"],
        "cis_controls_ig2": ig_parse_out["cis_controls_ig2"],
        "cis_controls_ig3": ig_parse_out["cis_controls_ig3"],
        "ig1": ig_parse_out["ig1"],
        "ig2": ig_parse_out["ig2"],
        "ig3": ig_parse_out["ig3"]
    }

    return control_data



def WriteControls(controls):

    global foundational_system
    now = datetime.now()

    filename=f'SSPHP Documentation/cis_benchmark_v8_doc_{foundational_system.lower()}.csv'

    with open(filename, "w") as f:

        # Write the file headings
        outline = f'\
"ssphp.cis_benchmark.control.number", \
"ssphp.use_case.foundational_system", \
"ssphp.use_case.id", \
"ssphp.use_case.savedsearch", \
"ssphp.use_case.title", \
"ssphp.cis_benchmark.control.title", \
"ssphp.cis_benchmark.control.profile_applicability", \
"ssphp.cis_benchmark.control.level", \
"ssphp.cis_benchmark.control.description", \
"ssphp.cis_benchmark.control.rationale", \
"ssphp.cis_benchmark.control.impact", \
"ssphp.cis_benchmark.control.group", \
"ssphp.cis_benchmark.controls.v8", \
"ssphp.cis_benchmark.controls.ig1", \
"ssphp.cis_benchmark.controls.ig2", \
"ssphp.cis_benchmark.controls.ig3", \
"ssphp.use_case.framework.ig_1", \
"ssphp.use_case.framework.ig_2", \
"ssphp.use_case.framework.ig_3", \
"ssphp.cis_benchmark.document.name", \
"ssphp.cis_benchmark.document.version", \
"ssphp.cis_benchmark.document.date", \
"ssphp.cis_benchmark.version", \
"ssphp.metadata.last_updated_by", \
"ssphp.metadata.last_updated_date"\
\n'
        f.write(outline)

        # Write the controls
        for control in controls:
            print(f'{control["id"]}   {control["title"]}')

            # Put together the line to write
            if foundational_system == "AZURE":
                benchmark_date = "2023-02-14"
                benchmark_file_description = "CIS Microsoft Azure Foundations Benchmark"
                benchmark_file_date = "2.0.0"
            elif foundational_system == "M365":
                benchmark_date = "2023-03-31"
                benchmark_file_description = "CIS Microsoft 365 Foundations Benchmark"
                benchmark_file_date = "2.0.0"
            elif foundational_system == "DNS":
                benchmark_date = "2023-06-28"
                benchmark_file_description = "CIS Amazon Web Services Foundations Benchmark"
                benchmark_file_date = "2.0.0"
            elif foundational_system == "GITHUB":
                benchmark_date = "2022-12-28"
                benchmark_file_description = "CIS GitHub Benchmark"
                benchmark_file_date = "1.0.0"
            elif foundational_system == "AKS":
                benchmark_date = "2025-04-16"
                benchmark_file_description = "CIS AKS Benchmark"
                benchmark_file_date = "1.7.0"
            else:
                benchmark_date = "-"

            outline = f'\
"{control["id"]}", \
"{foundational_system.upper()}", \
"{control["use_case_id"]}", \
"{control["use_case_savedsearch"]}", \
"{control["use_case_title"]}", \
"{control["title"]}", \
"{control["profile_applicability"]}", \
"{control["level"]}", \
"{control["description"]}", \
"{control["rationale"]}", \
"{control["impact"]}", \
"{control["section_header"]}", \
"{control["cis_controls"]}", \
"{control["ig1"]}", \
"{control["ig2"]}", \
"{control["ig3"]}", \
"{control["cis_controls_ig1"]}", \
"{control["cis_controls_ig2"]}", \
"{control["cis_controls_ig3"]}", \
"{benchmark_file_description}", \
"{benchmark_file_date}", \
"{benchmark_date}", \
"CIS v8", \
"Auto Document Extraction Script", \
"{now.strftime("%Y-%m-%d %H:%M:%S")}"\
\n'


            f.write(outline)

    return filename




def main():
    controls = []
    page_no = int(0)
    control_lines = ""
    first_time_thru = bool(True)
    start_page = 20
    end_page = 496
    toc_text = ""
    start_toc = False


    global foundational_system
    global sections


    if len(sys.argv) > 1:
        foundational_system = sys.argv[1]

    print(f'Processing Foundational System CIS Benchmark for {foundational_system}')

    if foundational_system == "DNS":
        filename = "SSPHP Documentation/CIS_Amazon_Web_Services_Foundations_Benchmark_v2.0.0.pdf"
    elif foundational_system == "GITHUB":
        filename = "SSPHP Documentation/CIS GitHub Benchmark v1.0.0 PDF.pdf"
    elif foundational_system == "AKS":
        filename = "SSPHP Documentation/CIS Azure Kubernetes Service (AKS) Benchmark v1.7.0.pdf"
    else:
        filename = f'SSPHP Documentation/CIS_Microsoft_{foundational_system.upper()[0]}{foundational_system.lower()[1:]}_Foundations_Benchmark_v2.0.0.pdf'

    print(f'Processing file {filename}')
    
    if foundational_system == "365":
        foundational_system = "M365"


    reader = PdfReader(filename)
    
    ########################### Extract the TOC ###############################################################################################
    for page in reader.pages:
        page_no = page.page_number

        page_text = re.sub(r"Internal Only - General\s*","",page.extract_text())

        match_page = re.match(r"\s*P\s*a\s*g\s*e\s*\d+\s*T\s*a\s*b\s*l\s*e\s*o\s*f\s*C\s*o\s*n\s*t\s*e\s*n\s*t\s*s\s*", page_text)
        if match_page:
            start_toc = True
            print(f'TOC starts at {page_no}')
            start_page = page_no

        match_page = re.match(r"\s*P\s*a\s*g\s*e\s+\d+\s+O\s*v\s*e\s*r\s*v\s*i\s*e\s*w", page_text)
        if match_page:
            print(f'TOC ends at {page_no}')
            end_page = page_no
            break
        
        if start_toc:
            toc_text = toc_text + page_text
    
    #print(toc_text)


    ########################### Process the TOC ###############################################################################################

    if foundational_system == "GITHUB":
        toc_text = re.sub(r"\.\.\.\s*(\d{1,3})",r" \1\n",toc_text)

    for line in toc_text.splitlines():

        #print(f'**{line}$$')

        if foundational_system == "DNS":
            match_line = re.match(r"1\.1\s[\s\S\.]*\s(?P<pno>\d+)\s*$", line)
        elif foundational_system == "AKS":
            match_line = re.match(r"\s*2\.1\.1\s[^.]*[\.\s]*\s(?P<pno>\d+)\s", line)
        else:
            match_line = re.match(r"\s*1\.1\.1\s[\s\S\.]*\s(?P<pno>\d+)\s*$", line)

        if match_line:
            start_page = int(match_line["pno"])

        match_line = re.match(r"\s*(?P<section_no>\d+)\s+(?P<section_name>[^\.]*)[\s\.]*(?P<pno>\d*)\s*$", line)
        
        if match_line:
            name = match_line['section_name'].rstrip()
            sections.append({"no": match_line['section_no'], "name": name})

        match_line = re.match(r"\s*A\s*p\s*p\s*e\s*n\s*d\s*i\s*x[\s\S\.]*\s(?P<pno>\d+)\s*$", line)
        if match_line:
            end_page = int(match_line["pno"])
            break
    
    print(f"Data starts at {start_page}")
    print(f"Data ends at {end_page}")



    for section in sections:
        print(f"Section {section['no']} is {section['name']}")



    ########################### Iterate through the pages and make a big array of control dicts for them all ###############################################################################################
    for page in reader.pages:

        # Get the page number for every page and throw away pages at start and end of the document
        page_no = page.page_number
        page_text = re.sub(r"Internal Only - General\s*","",page.extract_text())

        if page_no > start_page - 1 and page_no < end_page:

            # Is this page that starts a new control?
            match_start_page = re.match(r"\s*P\s*a\s*g\s*e\s+\d+\s+(?P<use_case_id>\d\d?\.\d\d?\.?\d?\d?)\s(?P<title>[\s\S]*)P\s*r\s*o\s*f\s*i\s*l\s*e\s+A\s*p\s*p\s*l\s*i\s*c\s*a\s*b\s*i\s*l\s*i\s*t\s*y\s*:", page_text)

            if match_start_page:
                if not first_time_thru:
                    controls.append(Parse_Control(control_text))

                control_text = ""
                first_time_thru = False

            control_text = control_text + page_text

    controls.append(Parse_Control(control_text))    # Add the final control


    ########################### Write the Output File ###############################################################################################
                
    outfile = WriteControls(controls)
    print(f'Output data to file {outfile}')





if __name__ == "__main__":
    main()
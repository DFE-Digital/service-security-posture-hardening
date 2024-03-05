#!/usr/bin/env python3
import os
import re


def ConvertMDtoHTML(inline):
    #inline = "----" + inline

    inline = re.sub(r"&","&amp;",inline)
    inline = re.sub(r"<","&lt;",inline)
    inline = re.sub(r">","&gt;",inline)

    m1 = re.match(r"#\s",inline)
    if m1:
        inline = re.sub(r"# ","<div style=\"font-size:175%;color:yellow;\">",inline)
        inline = re.sub(r"\n","",inline)
        inline = inline + "</div>"

    m2 = re.match(r"##\s",inline)
    if m2:
        inline = re.sub(r"## ","<div style=\"font-size:125%;color:CornflowerBlue;\">",inline)
        inline = re.sub(r"\n","",inline)
        inline = inline + "</div>"

    m3 = re.match(r".*\*\*(?P<bold_text>[^\*]*)\*\*",inline)
    if m3:
        bold_text = "<b>" + m3["bold_text"].strip() + "</b>"
        inline = re.sub(r"\*\*[^\*]*\*\*",bold_text,inline)

    m4 = re.match(r".*\*(?P<italic_text>[^\*]*)\*",inline)
    if m4:
        italic_text = "<i>" + m4["italic_text"].strip() + "</i>"
        inline = re.sub(r"\*[^\*]*\*",italic_text,inline)
        
    return inline



def GetMDFile(infile):

    in_txt = ""

    filename=os.path.dirname(__file__) + infile
    with open(filename, "r") as f2:
        for line in f2:
            in_txt = in_txt + ConvertMDtoHTML(line)

    in_txt = re.sub(r"\n\n","<br></br>",in_txt)
    in_txt = in_txt + "\n\n"
    return in_txt



def main():

    in_dash = ""
    link_file = ""

# Read the Dashboard Template File
    filename=os.path.dirname(__file__)+"\\SSPHP Documentation\\SSPHP_Metrics Documentation\\ssphp_help_dashboard_template.xml"
    with open(filename, "r") as f1:
        for line in f1:
            link_line = re.search(r"~~~(?P<link_file>.*)~~~",line)
            if link_line:
                link = link_line["link_file"]
                line = GetMDFile(link)

            in_dash = in_dash + line


# write the actual Dashboard File
    #print(in_dash)


    filename=os.path.dirname(__file__)+"\\SSPHP\\SSPHP_metrics\\default\\data\\ui\\views\\ssphp_foundational_systems_dashboard_help.xml"
    with open(filename, "w") as f:
        f.write(in_dash)



if __name__ == "__main__":
    main()

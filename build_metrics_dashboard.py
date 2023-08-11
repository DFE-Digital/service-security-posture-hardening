#!/usr/bin/env python3
import os
import re


def main():

    # print("started\n")
# Increment the version in the first file
    out_string=""
    version_string="Service Security Posture Hardening Programme : v"

    filename=os.path.dirname(__file__)+"\\SSPHP\SSPHP_metrics\\default\\data\\ui\\views\\ssphp_metrics_dashboard.d\\ssphp_metrics_dashboard_block_1.xml"
    with open(filename, "r") as f:
        for x in f:
            if version_string in x:
                #print(x)
                v=re.search(r' : v(\d*\.\d*)\<\/description\>', x)
                version_old=float(v.group(1))
                version_new=str((f'{version_old+0.01:.2f}'))
                version_old=str(f'{version_old:.2f}')
                x=x.replace(version_old,version_new)

                #print(x)
            out_string=out_string+x

    with open(filename, "w") as f:
        for x in out_string:
          f.write(x)  


# Process the files

    out_string=""

    filename=os.path.dirname(__file__)+"\\SSPHP\SSPHP_metrics\\default\\data\\ui\\views\\ssphp_metrics_dashboard.d\\ssphp_metrics_dashboard_block_1.xml"
    with open(filename, "r") as f:
        for x in f:
            out_string=out_string+x
          
    out_string=out_string+"\n\n\n"
    
    iv=[{"detail":"dns","heading":"DNS"},{"detail":"aad","heading":"Azure Active Directory"},{"detail":"azr","heading":"Azure"},{"detail":"m365","heading":"Office 365"},{"detail":"busc","heading":"Business Central"}]
    for service_name in iv:
        filename=os.path.dirname(__file__)+"\\SSPHP\SSPHP_metrics\\default\\data\\ui\\views\\ssphp_metrics_dashboard.d\\ssphp_metrics_dashboard_block_2.xml"
        with open(filename, "r") as f:
            for x in f:
              x=x.replace("~~~~",service_name["detail"])
              x=x.replace("^^^^",service_name["heading"])
              out_string=out_string+x

        out_string=out_string+"\n\n\n"
    

    filename=os.path.dirname(__file__)+"\\SSPHP\SSPHP_metrics\\default\\data\\ui\\views\\ssphp_metrics_dashboard.d\\ssphp_metrics_dashboard_block_3.xml"
    with open(filename, "r") as f:
        for x in f:
            out_string=out_string+x
          
    out_string=out_string+"\n\n\n"          
          

    filename=os.path.dirname(__file__)+"\\SSPHP\SSPHP_metrics\\default\\data\\ui\\views\\ssphp_metrics_dashboard.xml"
    with open(filename, "w") as f:
        for x in out_string:
          f.write(x)      

    print("\ndone - output file was "+filename+"\nnew version="+version_new)


if __name__ == "__main__":
    main()

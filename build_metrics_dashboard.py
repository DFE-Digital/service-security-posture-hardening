#!/usr/bin/env python3
import os


def main():

    # print("started\n")
    
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

    print("\ndone - output file was "+filename)

if __name__ == "__main__":
    main()

import sys
import os

del sys.path[-1]
dir_path = os.path.dirname(os.path.realpath(__file__))

sys.path.insert(0, f"{dir_path}/aws")
sys.path.insert(0, f"{dir_path}/ms_graph")
sys.path.insert(0, f"{dir_path}/.python_packages/lib/site-packages")

import aws_data_ingester
import ms_graph_data_ingester
import asyncio

def main():
    asyncio.run(aws_data_ingester.main(None))
    asyncio.run(ms_graph_data_ingester.main(None))    

if __name__=="__main__":
    main()

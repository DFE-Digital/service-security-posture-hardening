# BACKGROUND


## CISD



## ATO
DfE technology platforms and services have always been given an 'Authority to Operate' certfification based on a period 'Audit' that security and other criteria and standards have been complied with. The ATO 'Audit' process is paper-based and led by an ISO in CISD who acts as the interface to the service owner and SRO. Depending on the importance of the system or service - Gold, Silver, Bronze - the ATO process is repeated every 1 - 3 years. Clearly, it is essential that during the intervening period the ISO is kept abreast of any changes to the service that might potentially have impact on the security of the systems, but implicitly under this scheme confidence in security can only be as good as it was at the time of the most recent audit.


## Continuous Assurance
Continuous Assurance is a DfE CISD initiative aimed at **constant automated** measuring of the Compliance of key technologies against documented Policies. The objective is to minimise the possibility of key infrastructure and services being compromised, by quickly identifying, remediating and mitigating any compliance breaches and divergence from Policies and standards - otherwise known as Security Posture Hardening. 

The route is to check configuration against pre-selected policies, architectures and settings, where possible using industry standards and recognised best practice. This gives Service Owners a daily view of status, and a management to track compliance, risk and progress on mitigation.


## Security Posture Hardening
This focus on 'Protect' is a key tool in preventing compromises by making it difficult for a prospective attacker to find a way into the systems. Other CISD initiatitives concentrate on other phases of the pcess, such 'Detect' (identifying signs that a compromise has occured, which happens in the SOC), 'Respond', and 'Recover'.


## Policies
Policies are documented, and owned by..... Many are 



## Benchmarks
Industry Standard Benchmarks are hugely valuable asset for Posture Hardening because it allows DfE to make rely on global expertise while saving the time and resources to develop Policies in-house, meaning that the time to market is much smaller. Also, it is a far better approach to make a single decision around which benchmark to choose than to defend each individual decision for every single policy and implementation.

*CIS - Center for Internet Security* - has a wealth of industry benchmarks for all of the major cloud-based technologies. The documentation can be found [here](https://downloads.cisecurity.org/#/). One of the great positives of the CIS Benchmarks is that they are incredibly detailed and prescriptive in what needs to happen in order to audit and remediate, the priorities and the impacts. Where possible, the CIS Benchmarks have been used; where there is no CIS Benchmark available, a custom set of Policies will have been developed by CISD having Threat Modelled the target system.


# CONTROLS

A 'Control' or 'Use Case' is the manifestation, in Splunk, of a specific Policy against which Compliance is being measured. So Controls, Use Cases, and Policies are effectively all the same thing. For every Control, the output from running the control is either 'Compliant' or 'Non-Compliant'.


## Scores
In order to be more useful, the output from a Control is actually a 'Score'; a score being more granular than simply reporting 'Compliant' or Non-Compliant', since it can hint at the degree of non-compliance. Scores will mean different things in different contexts, but throughout SSPHP a score of 100 is perfectly compliant, and a score of 0 is the worst possible where every single test failed. Scores between 0 and 100 are non-compliant, but the closer to 100 the better.

For each Control, the Score is calculated from a 'Numerator' and a 'Denominator'. The denominator is the number of things tested, and the numerator is the number of those tests which **failed**.

There are 2 types of Controls - those which require a single setting or collection of settings to be a certain way for the system or service as a whole, and those which require many users or resources to each have the specified settings. For the former, the denominator is the number of fields that were tested and the numerator is the number of fields that failed the tests. For the latter, each line is either a pass or a fail based on whether it contains 1 or more fields which failed a test, the denominator is the total number of lines that were tested, and the numerator is the number of lines that failed one or more tests.


## DfE Mandated Controls
Every Control has been assigned a level of priority within 3 bands - DfE Mandated, Recommended, Desirable. The designation is largely based on that in the Benchmark, but some have been moved depending on CISD's interpretation of risk based on DfE's usage.

In the first phase, System and Service Owners are expected to be compliant with all of the DfE Mandated Policies/Controls that relate to their particular technology. Where they are not, they are expected to remediate, or to demonstrate to the satisfaction of the CISO that the associated risk has been mitigated in a different way. It is very much the intent of CISD to work with Service Owners to ensure that risk is mitigated and their service status is Compliant.

In later phases and over time, all services will be expected to become compliant with the Controls in all 3 bands.


## Naming of Controls
The name of the Control is based on the source system and the naming in CIS Benchmark v8. 

** NOTE ** Once developed, the Controls will not change name, even if CIS renumber their documents in future versions of the Benchmark. The dashboards will display the equivalent new number for reference, but the Control will retain its original code. For example, the Control named 'M365 001 [CIS 1.1.2]' relates to M365, is in category 1 in the document which relates to "Account / Authentication", and is control 1.1.2.


## Cadence
The configuration data from the underlying systems and services is requested every day at 3am, and sent to Splunk. The Control algorithms run during a period between 4am and 7am.

So the data in the dashboards is updated on a daily basis.
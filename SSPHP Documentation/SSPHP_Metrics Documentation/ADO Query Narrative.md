Description of the ADO Use Case Query
=====================================


Notes :	Organisations can have many projects, and projects can have many repos. 
		Projects can only exist inside an Organisation, and Repos can only exist inside a project.

		Policies exist as seperate entities from Organisations, Projects and Repos. But each policy is specific to a single organisation and project. The data inside the policy definition record may also make it specific to a single repo, otherwise the policy applies to all repos in the project. 

		Each Repo may have a number of policies of a single type, each of which has a slightly different scope. There is a rule of precedence over which policy will apply when there is more than 1.... the most secure policy will prevail. So, for example, if there is a policy for a project that says there must be 2 approvers, and another policy which refers specifically to a repo that lives in that project and says that there must be 3 approvers, then for that repo the 3 rule will apply and for all other repos in the project the 2 rule will apply. If however, the project policy says that there must be 3 approvers, then that will apply to every repo in the project unless one has a higher requirement set for that repo alone.

		The use case is written such that everything above the line ``` ===================== vvvv USE CASE SPECIFIC vvvv ===================== ``` is generic and should be able to be used in all use cases that rely on this dataset. Below this line is the code which should be use case specific.


The Query :

1. The main body of the search gets the list of Repositories. The objective is to enrich each repo with a single value for the precedent policy to which the use case refers. In this case it's the minimum approver count. This value will be returned by the join, connected through the organisation, project, and repo.


2. ``` ***************** JOIN 1 ***************** ```
   gets the list of policies and data associated with each policy - the settings / rules. Each policy carries with it the Organisation and Project to which it applies.

   The problem is that we need to match by repository in order to do our tests.


3. ``` ***************** JOIN 2 ***************** ```
   so this join (which lives inside the join above), enriches the data with a complete list of all the Repos that exist for the Organisation / Project. The mvexpand command then creates a single event for every repo in a project, where all fields for the policy are identical except the repo.

   At this point we have policies enriched with the repos to which they may apply, but do not necessarily apply. So we filter out the policy records which do not apply to the repo o which they are joined.


So the join ends up returning policy data for each repo.


4. ``` ===================== vvvv USE CASE SPECIFIC vvvv ===================== ```
   from here we are looking for narrow down to a single policy, which is the one we are testing - we do this using the UUID for the template rather than the policy name, since the names are admin defined but may refer to the same policies.

   the final thing is to stats together the data from all the policy records for that policy for the repo for that project for that organisation. There will need to be some logic bespoke to each use case to decide what is the final value from precedence. For example. for number of approvers it will be whichever applicable policy has the highest number as the setting. For a different use case it might be true if any of the applicable policies reurn true. potentially, for other use cases, it may be that all poicies must show true.
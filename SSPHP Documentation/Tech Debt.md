Tech Debt

There are a number of areas that the app has known weaknesses or issues that will need to be addressed in the future :

- Population of the Asset / Service lookup map : the search logic that assigns Azure resources to a DfE Service is born in the imaginaion of Ian Pearl and Alex Kinnane. The top level objective of the project has always been to provide DfE Services with the things that they should remediate. In lieu of any kind of map of the organisation, we invented our own. But it is very definitely not going to be very good, and we are going to need to write a beter model, but the current model is better than the none that existed before. Plus the model has enabled us to move forward with things like mapping IP addresses to Services.


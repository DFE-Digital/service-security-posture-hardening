# Infrastructure as Code for Data Ingester

We use Terraform to deploy our infrastructure.

The supported version  is stored in `.terraform-version`.

# Structure

The [data_ingester](data_ingester/README.md) directory contains a parameterized Terraform module.

The `red` directory contains our production deployment.

The `blue` directory contains our development deployment.

TODO: REMOVE `state_terraform`

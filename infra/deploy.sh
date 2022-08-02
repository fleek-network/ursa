#!/bin/sh

# Create a new instance with new ssh key
# place the ssh key into the user-data
ssh-keygen -t rsa -C "mahmoud@fleek.co" -f ./tf-digitalocean 

# Import env vars for a single node
source env.sh

###########
#   TF    #
###########
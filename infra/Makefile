.PHONY: init
init:
	@cd tf-next && terraform init
	@cp tf-next/production.tfvars.example tf-next/production.tfvars

.PHONY: plan
plan:
	@cd tf-next && terraform plan -var-file="production.tfvars"

.PHONY: apply
apply:
	@cd tf-next && terraform apply -auto-approve -var-file="production.tfvars"

.PHONY: destroy
destroy:
	@cd tf-next && terraform destroy -var-file="production.tfvars"
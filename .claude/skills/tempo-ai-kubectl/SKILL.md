---
name: tempo-ai-kubectl
description: Read-only kubectl operations on non-production Tailscale clusters. Use when asked about pods, deployments, logs, or cluster resources in dev/staging environments.
---

# Kubectl Read-Only Operations

Query Kubernetes clusters via Tailscale. **Read-only operations only** - no create, delete, apply, or edit commands.

## Prerequisites

- Connected to Tailscale
- kubectl installed

## Setup Check

First, check if kubeconfig is configured:

```bash
kubectl config get-contexts
```

If no Tailscale clusters appear (e.g., `dev-euw.tail388b2e.ts.net`), tell the user to configure:

```bash
tailscale configure kubeconfig <cluster-name>
# Example: tailscale configure kubeconfig dev-euw
```

## Allowed Clusters

**ONLY use these clusters** (exclude `prd-*` and `ic-2-*`):
- `dev-euw.tail388b2e.ts.net` - Development EU West
- `stg-nae.tail388b2e.ts.net` - Staging NA East
- `ic-1-tailscale-operator.tail388b2e.ts.net` - IC-1 cluster

If user asks about production (`prd-*`) or `ic-2-*`, **refuse** and explain these are excluded for safety.

## Commands (Read-Only)

**ALWAYS specify `--context`** on every command:

```bash
# List resources
kubectl --context=dev-euw.tail388b2e.ts.net get pods -A
kubectl --context=dev-euw.tail388b2e.ts.net get deployments -n <namespace>
kubectl --context=dev-euw.tail388b2e.ts.net get services -n <namespace>
kubectl --context=dev-euw.tail388b2e.ts.net get nodes

# Describe resources
kubectl --context=dev-euw.tail388b2e.ts.net describe pod <pod> -n <namespace>
kubectl --context=dev-euw.tail388b2e.ts.net describe deployment <deploy> -n <namespace>

# Logs
kubectl --context=dev-euw.tail388b2e.ts.net logs <pod> -n <namespace>
kubectl --context=dev-euw.tail388b2e.ts.net logs <pod> -n <namespace> --tail=100
kubectl --context=dev-euw.tail388b2e.ts.net logs -l app=<label> -n <namespace>

# Events
kubectl --context=dev-euw.tail388b2e.ts.net get events -n <namespace> --sort-by='.lastTimestamp'

# Resource details
kubectl --context=dev-euw.tail388b2e.ts.net get pod <pod> -n <namespace> -o yaml
kubectl --context=dev-euw.tail388b2e.ts.net top pods -n <namespace>
kubectl --context=dev-euw.tail388b2e.ts.net top nodes
```

## Forbidden Commands

**NEVER run these commands:**
- `kubectl apply`
- `kubectl create`
- `kubectl delete`
- `kubectl edit`
- `kubectl patch`
- `kubectl replace`
- `kubectl scale`
- `kubectl rollout restart`
- `kubectl exec`
- `kubectl port-forward`
- Any command that modifies cluster state

If user requests a write operation, explain this skill is read-only and suggest using ArgoCD for deployments.

## Workflow

1. Run `kubectl config get-contexts` to check available clusters
2. If Tailscale clusters missing, prompt user to run `tailscale configure kubeconfig <cluster>`
3. If user mentions "dev" → use `dev-euw.tail388b2e.ts.net`
4. If user mentions "staging" → use `stg-nae.tail388b2e.ts.net`
5. If user asks about "prod" or "ic-2" → **refuse**, explain exclusion

## Examples

```bash
# Check pods in dev
kubectl --context=dev-euw.tail388b2e.ts.net get pods -A | grep -v Running

# Get logs from staging
kubectl --context=stg-nae.tail388b2e.ts.net logs -l app=my-service -n default --tail=50

# Describe failing pod
kubectl --context=dev-euw.tail388b2e.ts.net describe pod crash-loop-pod -n my-namespace

# Check node resources
kubectl --context=dev-euw.tail388b2e.ts.net top nodes
```

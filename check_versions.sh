#!/bin/bash

echo "=== Checking Git Commit Versions of Running Containers ==="
echo

# Check solver container
echo "Solver Container:"
docker inspect vamp_fun_solver --format='{{.Config.Labels.git_commit}}' 2>/dev/null || echo "Container not found or no git commit label"
echo

# Check request registrator container
echo "Request Registrator Container:"
docker inspect vamp_fun_request_registrator_ethereum --format='{{.Config.Labels.git_commit}}' 2>/dev/null || echo "Container not found or no git commit label"
echo

# Check environment variables too
echo "Solver Environment Variable:"
docker exec vamp_fun_solver env | grep GIT_COMMIT 2>/dev/null || echo "Container not found or no GIT_COMMIT env var"
echo

echo "Request Registrator Environment Variable:"
docker exec vamp_fun_request_registrator_ethereum env | grep GIT_COMMIT 2>/dev/null || echo "Container not found or no GIT_COMMIT env var"
echo

# Note: This script runs on deployment server which doesn't have git access
echo "Note: Git commit information comes from Docker image labels and environment variables"
echo

echo "=== Container Details ==="
docker ps --format "table {{.Names}}\t{{.Image}}\t{{.Status}}" | grep -E "(solver|registrator|validator)"

echo
echo "=== All Container Labels ==="
echo "Solver labels:"
docker inspect vamp_fun_solver --format='{{range $k, $v := .Config.Labels}}{{$k}}={{$v}}{{"\n"}}{{end}}' 2>/dev/null || echo "Container not found"

echo "Request Registrator labels:"
docker inspect vamp_fun_request_registrator_ethereum --format='{{range $k, $v := .Config.Labels}}{{$k}}={{$v}}{{"\n"}}{{end}}' 2>/dev/null || echo "Container not found"

echo "Validator labels:"
docker inspect vamp_fun_validator_vamp --format='{{range $k, $v := .Config.Labels}}{{$k}}={{$v}}{{"\n"}}{{end}}' 2>/dev/null || echo "Container not found" 
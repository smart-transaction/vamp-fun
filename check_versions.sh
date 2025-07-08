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

# Show current git commit
echo "Current Git Commit:"
git rev-parse HEAD
echo

echo "=== Container Details ==="
docker ps --format "table {{.Names}}\t{{.Image}}\t{{.Status}}" | grep -E "(solver|registrator)" 
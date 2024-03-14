#!/bin/bash

# Only run this on AWS agents
if [[ -z "${BUILDKITE_AGENT_META_DATA_AWS_REGION}" ]]; then
	exit 0
fi

mkdir -p logs

echo "--- :docker: Saving Docker Logs"
docker-compose logs wpcli > logs/wpcli.log
docker-compose logs wordpress > logs/wordpress.log
docker-compose logs mysql > logs/mysql.log

buildkite-agent artifact upload "logs/*.log"

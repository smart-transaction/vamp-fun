FROM --platform=linux/amd64 mysql:8.0

USER root
COPY ./setup.sql /docker-entrypoint-initdb.d/setup.sql
COPY ./init.sql .
COPY ./set_passwords.sh .

EXPOSE 3306/tcp
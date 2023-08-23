FROM python:3.8

ADD . /opt/app
RUN pip install -v /opt/app

EXPOSE 8001
CMD huggingface-cli login --token $HUGGINGFACE_TOKEN && python -m code_scratchpads.http_server

# docker build -t europe-west4-docker.pkg.dev/small-storage1/databases-and-such/code-scratchpads:20230823v2 .

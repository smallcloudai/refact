FROM python:3.8

ADD . /opt/app
RUN pip install -v /opt/app

EXPOSE 8008
CMD huggingface-cli login --token $HUGGINGFACE_TOKEN && python -m code_scratchpads.http_server

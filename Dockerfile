FROM python:3.8

ADD . /opt/app
RUN pip install -v /opt/app

EXPOSE 8001
CMD python -m code_scratchpads.http_server

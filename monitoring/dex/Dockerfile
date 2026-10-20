FROM python:3.14.5-alpine3.22@sha256:6b91e66ab2a880ce9ca5a1b91c70f45963ff71ff68268df056336e1a657d5efd
WORKDIR /app
COPY monitoring/dex/exporter.py monitoring/dex/drill.py /app/
RUN mkdir /state && chown 65534:65534 /state
USER 65534:65534
EXPOSE 9800 9801
CMD ["python", "-I", "/app/exporter.py"]

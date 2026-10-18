FROM python:3.13.9-alpine3.22@sha256:e5fa639e49b85986c4481e28faa2564b45aa8021413f31026c3856e5911618b1
WORKDIR /app
COPY monitoring/dex/exporter.py monitoring/dex/drill.py /app/
RUN mkdir /state && chown 65534:65534 /state
USER 65534:65534
EXPOSE 9800 9801
CMD ["python", "-I", "/app/exporter.py"]

#!/bin/sh

echo "Downloading data"
curl https://teeline.s3.eu-central-1.amazonaws.com/data.zip -o data.zip

echo "Unpacking archive"
unzip data.zip

echo "Removing archive"
rm data.zip

echo "Done!"


#!/usr/bin/env bash

set -eux

mkdir -p models
wget https://huggingface.co/pan93412/yolo-v11-onnx/resolve/main/yolo11x.onnx -O models/yolo11x.onnx

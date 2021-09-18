import tensorflow as tf
from tensorflow.keras import layers
from tensorflow.keras.layers.experimental import preprocessing
from tensorflow.keras.models import load_model

import pathlib
import os
import sys
import matplotlib.pyplot as plt
import pandas as pd
import numpy as np

# argv usage: [symbol, interval, numcandles]
symbol = sys.argv[1]
interval = sys.argv[2]
candles_forward = sys.argv[3]

model_path = os.path.join(
    'builder', 'models', symbol, interval, candles_forward, 'model')
model = load_model(model_path)
csv_data = np.array(pd.read_csv(
    "builder/csv/predict/predict.csv", header=None))
csv_data = csv_data.ravel()
csv_data = csv_data[
    np.logical_not(np.isnan(csv_data))]

csv_data = np.array([csv_data]).astype("float32")
predictions = model.predict(csv_data)

f = open("prediction", "w")
f.write(str(predictions[0][0]))
f.close()
print(predictions)

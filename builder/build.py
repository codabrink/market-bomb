import tensorflow as tf
from tensorflow.keras import layers
from tensorflow.keras.layers.experimental import preprocessing

import pathlib
import os
import sys
import matplotlib.pyplot as plt
import pandas as pd
import numpy as np
import shutil


# argv usage: [symbol, interval, numcandles]

np.set_printoptions(precision=4)

num_files = sum([len(files)
                 for r, d, files in os.walk("./csv")])
file_index = 0

labels = []
features = []

symbol = sys.argv[1]
interval = sys.argv[2]
candles_forward = sys.argv[3]

if os.path.exists('labels.npy'):
    labels = np.load('labels.npy')
    features = np.load('features.npy')
else:
    with os.scandir(os.path.join('csv', 'train', symbol, interval, candles_forward)) as folder:
        for csv in folder:
            print("Loading: " + str(int(file_index / num_files * 100)) + "%")
            # print("filename: " + csv.name)
            label = float(csv.name.split(",")[1].split('.csv')[0])
            # print("label: " + str(label))
            labels.append(label)
            csv_data = np.array(pd.read_csv(csv, header=None))
            csv_data = csv_data.ravel()
            csv_data = csv_data[
                np.logical_not(np.isnan(csv_data))]
            features.append(csv_data)
            file_index += 1
    np.save('labels', labels)
    np.save('features', features)

labels = np.asarray(labels).astype('float32')
features = np.asarray(features).astype('float32')

print(features)
assert not np.any(np.isnan(features))
assert not np.any(np.isnan(labels))

print("Building model.")
# model = tf.keras.
model = tf.keras.Sequential([
    layers.Dense(128, activation='relu'),
    layers.Dense(64),
    layers.Dense(128),
    layers.Dense(32, activation='relu'),
    layers.Dense(1, activation='linear')
])

opt = tf.keras.optimizers.SGD(
    learning_rate=0.01, momentum=0.0, nesterov=False, name='SGD'
)

print("Compiling model.")
model.compile(loss=tf.losses.MeanSquaredError())
# optimizer=opt)

print("Fitting model.")
model.fit(features, labels, epochs=170)

model_path = os.path.join('models', symbol, interval, candles_forward)
shutil.rmtree(model_path, ignore_errors=True)
os.makedirs(model_path)
model.save(os.path.join(model_path, 'model'))

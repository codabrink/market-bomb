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

physical_devices = tf.config.list_physical_devices('GPU')
tf.config.experimental.set_memory_growth(physical_devices[0], True)

# argv usage: [symbol, interval, numcandles]

np.set_printoptions(precision=4)

num_files = sum([len(files)
                 for r, d, files in os.walk("./csv")])
file_index = 0

labels = []
features = []
files = []

test_labels = []
test_features = []

symbol = sys.argv[1]
strat = sys.argv[2]

if False: #os.path.exists('labels.npy'):
    labels = np.load('labels.npy')
    features = np.load('features.npy')
else:
    with os.scandir(os.path.join('csv', symbol, strat, 'train')) as folder:
        for csv in folder:
            print("Loading: " + str(int(file_index / num_files * 100)) + "%")
            files.append(csv.name)
            csv_data = np.array(pd.read_csv(csv, header=None))
            label = csv_data[-1][0]
            csv_data = np.delete(csv_data, len(csv_data) - 1, 0)
            # csv_data = csv_data.ravel()
            features.append(np.array(csv_data))
            labels.append(label)
            file_index += 1
    np.save('labels', labels)
    np.save('features', features)

file_index = 0
with os.scandir(os.path.join('csv', symbol, strat, 'test')) as folder:
    for csv in folder:
        print("Loading: " + str(int(file_index / num_files * 100)) + "%")
        csv_data = np.array(pd.read_csv(csv, header=None))
        label = csv_data[-1][0]
        csv_data = np.delete(csv_data, len(csv_data) - 1, 0)
        # csv_data = csv_data.ravel()
        test_features.append(np.array(csv_data))
        test_labels.append(label)
        file_index += 1

labels = np.asarray(labels).astype('float32')
features = np.asarray(features).astype('float32')
test_labels = np.asarray(test_labels).astype('float32')
test_features = np.asarray(test_features).astype('float32')

# print(features[0])
# for i in range(len(features)):
    # print("Checking " + files[i])
    # for ii in range(len(features[i])):
        # print("Checking row " + str(ii))
        # assert not np.any(np.isnan(features[i][ii]))
    # print(str(files[i]) + " is okay")
    
assert not np.any(np.isnan(features))
assert not np.any(np.isnan(labels))
assert not np.any(np.isnan(test_features))
assert not np.any(np.isnan(test_labels))

print("Building model.")
dropout = 0.3

model = tf.keras.Sequential()
model.add(tf.keras.Input(shape=(None, 5)))
model.add(
    layers.Bidirectional(
        layers.LSTM(256, return_sequences=True, activation='tanh')
    )
)
model.add(layers.Dropout(dropout))
model.add(
    layers.Bidirectional(
        layers.LSTM(256, activation='tanh')
    )
)
# model.add(layers.LSTM(256, activation='tanh'))
model.add(layers.Dropout(dropout))
model.add(layers.Dense(1, activation="linear"))


opt = tf.keras.optimizers.SGD(
    learning_rate=0.01, momentum=0.0, nesterov=False, name='SGD'
)

print("Compiling model.")
model.compile(
    loss="mean_absolute_error",
    # optimizer="adam",
    optimizer="rmsprop",
    metrics=["mean_absolute_error"]
    # metrics=["accuracy"]
)

print("Fitting model.")
model.fit(features, labels, epochs=200)

model.evaluate(test_features, test_labels, verbose=2)

model.save(os.path.join('saved_model/my_model'))
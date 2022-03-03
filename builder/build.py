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

np.random.seed(314)
tf.random.set_seed(314)
# random.seed(314)

# argv usage: [symbol, interval, numcandles]

np.set_printoptions(precision=4)

num_files = sum([len(files)
                 for r, d, files in os.walk("./csv")])
file_index = 0

symbol = sys.argv[1]

train_features = pd.read_csv(os.path.join('csv', symbol, 'train.csv'))
test_features = pd.read_csv(os.path.join('csv', symbol, 'test.csv'))

train_labels = train_features.pop("pct_change")
test_labels = test_features.pop("pct_change")

for i, v in enumerate(train_labels):
    if v == "pos":
        train_labels[i] = 0.99
    elif v == "neg":
        train_labels[i] = 0.
    else:
        train_labels[i] = .5

for i, v in enumerate(test_labels):
    if v == "pos":
        test_labels[i] = 0.99
    elif v == "neg":
        test_labels[i] = 0.
    else:
        test_labels[i] = .5

train_labels = np.asarray(train_labels).astype('float32')
test_labels = np.asarray(test_labels).astype('float32')


print(test_labels)

print("Building model.")
normalize = layers.Normalization()

# model = tf.keras.
model = tf.keras.Sequential([
    # layers.Normalization(),
    tf.keras.Input(shape=(None, 2567)),
    layers.LSTM(256, return_sequences=True),
    layers.Dropout(0.3),
    layers.LSTM(256, return_sequences=True),
    layers.Dropout(0.3),
    layers.LSTM(256, return_sequences=False),
    layers.Dropout(0.3),
    layers.Dense(1, activation="linear")
])

# opt = tf.keras.optimizers.SGD(
    # learning_rate=0.01, momentum=0.0, nesterov=False, name='SGD'
# )
# opt = optimizer = tf.optimizers.Adam()


print("Compiling model.")
model.compile(
    loss="mean_absolute_error",
    # loss="mse", 
    optimizer="rmsprop",
    metrics=["mean_absolute_error"]
)

print("Fitting model.")
model.fit(
    train_features,
    train_labels,
    batch_size=10,
    epochs=10
)
## validation_data=(test_features, test_labels)

for i in range(20):
    test = tf.constant([test_features.iloc[i]])
    print(test)
    prediction = model.predict(test)
    print(prediction)
    print("Result: " + str(test_labels[i]))
# print(test_labels)

# evaluation = model.evaluate(test_features, test_labels)
# print(evaluation)
# print(f"BinaryCrossentropyloss: {evaluation[0]}")
# print(f"Accuracy: {evaluation[1]}")

# model_path = os.path.join('models', symbol, interval, candles_forward)
# shutil.rmtree(model_path, ignore_errors=True)
# os.makedirs(model_path)
# model.save(os.path.join(model_path, 'model'))

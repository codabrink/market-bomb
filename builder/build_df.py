import tensorflow as tf
import tensorflow_decision_forests as tfdf
import os
import sys
import matplotlib.pyplot as plt
import pandas as pd
import numpy as np

np.set_printoptions(precision=4)

symbol = sys.argv[1]
train_df = pd.read_csv(os.path.join('csv', symbol, 'train.csv'))
test_df = pd.read_csv(os.path.join('csv', symbol, 'test.csv'))

train_ds = tfdf.keras.pd_dataframe_to_tf_dataset(train_df, label="pct_change")
test_ds = tfdf.keras.pd_dataframe_to_tf_dataset(test_df, label="pct_change")

print("Building model.")
model = tfdf.keras.RandomForestModel()
model.fit(x=train_ds)

model.compile(metrics=["accuracy"])
evaluation = model.evaluate(test_ds)

print(f"BinaryCrossentropyloss: {evaluation[0]}")
print(f"Accuracy: {evaluation[1]}")

model.save("forest")
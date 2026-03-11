import os


data_dir = os.path.join(os.getcwd(), "data_dir")
os.makedirs(data_dir, exist_ok=True)
rt_dir = os.path.join(data_dir, "rt_data")
static_file = os.path.join(data_dir, "static.db")
os.makedirs(rt_dir, exist_ok=True)

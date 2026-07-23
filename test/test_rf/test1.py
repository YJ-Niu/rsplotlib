import rsplotlib.pyplot as plt
import rsnumpy as np
from rsnumpy import absolute, log10, real, sum
from scipy.optimize import minimize

import skrf as rf

# 保存当前图
def ssaver(name):
    plt.savefig(name)
    plt.clf()


rf.stylely()
MSL100_raw = rf.Network('MSL100.s2p')
MSL200_raw = rf.Network('MSL200.s2p')

# Keep only the data from 1MHz to 5GHz
MSL100 = MSL100_raw['1-5000mhz']
MSL200 = MSL200_raw['1-5000mhz']

plt.figure()
plt.title('Measured data')
MSL100.plot_s_db()
MSL200.plot_s_db()
ssaver('./test/test1/test1.png')

import rsplotlib.pyplot as plt
import rsnumpy as np
from rsnumpy import absolute, log10, real, sum
from scipy.optimize import minimize
from skrf.media import MLine

import skrf as rf

# 保存当前图
def ssaver(name):
    plt.savefig(name)
    plt.clf()

def pprint(n, ss):
    print(f"Network {n}")
    print("++++++++++++++++++++++++++++++")
    print(ss, "\n")


plt.figure()
MSL100_raw = rf.Network('./test/test_rf/skrf/data/MSL100.s2p')
MSL200_raw = rf.Network('./test/test_rf/skrf/data/MSL200.s2p')

# # Keep only the data from 1MHz to 5GHz
MSL100 = MSL100_raw['1-5000mhz']
MSL200 = MSL200_raw['1-5000mhz']


plt.title('Measured data')
MSL100.plot_s_db()
MSL200.plot_s_db()
# rf.stylely()
ssaver('./test/test_rf/test_data/test1.png')

c0 = 3e8
f = MSL100.f
deltaL = 0.1
deltaPhi = np.unwrap(np.angle(MSL100.s[:, 1, 0])) - np.unwrap(np.angle(MSL200.s[:, 1, 0]))
Er_eff = np.power(deltaPhi * c0 / (2 * np.pi * f * deltaL), 2)
Loss_mea = 20 * log10(absolute(MSL200.s[:, 1, 0] / MSL100.s[:, 1, 0]))

plt.figure()
plt.suptitle('Effective relative permittivity and loss')
plt.subplot(2, 1, 1)
plt.plot(f * 1e-9, Er_eff)
plt.ylabel(r'$\epsilon_{r,eff}$')

plt.subplot(2, 1, 2)
plt.plot(f * 1e-9, Loss_mea)
plt.xlabel('Frequency (GHz)')
plt.ylabel('Insertion Loss (dB)')
ssaver('./test/test_rf/test_data/test2.png')

W = 3.00e-3
H = 1.55e-3
T = 50e-6
L = 0.1
Er0 = 4.5
tand0 = 0.02
f_epr_tand = 1e9
x0 = [Er0, tand0]

def model(x, freq, Er_eff, L, W, H, T, f_epr_tand, Loss_mea):
    ep_r = x[0]
    tand = x[1]
    m = MLine(frequency=freq, z0_port=50, w=W, h=H, t=T,
              ep_r=ep_r, mu_r=1, rho=1.712e-8, tand=tand, rough=0.15e-6,
              f_low=1e3, f_high=1e12, f_epr_tand=f_epr_tand,
              diel='djordjevicsvensson', disp='kirschningjansen')
    DUT = m.line(L, 'm')
    Loss_mod = 20 * log10(absolute(DUT.s[:, 1, 0]))
    return sum((real(m.ep_reff_f) - Er_eff)**2) + 0.01*sum((Loss_mod - Loss_mea)**2)


# res = minimize(model, x0, args=(MSL100.frequency, Er_eff, L, W, H, T, f_epr_tand, Loss_mea),
#                bounds=[(4.2, 4.7), (0.001, 0.1)])
# Er = res.x[0]
# tand = res.x[1]

# pprint(1, f'Er={Er:.3f}, tand={tand:.4f} at {f_epr_tand * 1e-9:.1f} GHz.')

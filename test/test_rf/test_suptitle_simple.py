import rsnumpy as np
import rsplotlib.pyplot as plt

freq = np.linspace(1, 2, 101)
s11 = 0.1 * np.exp(1j * freq * np.pi)
s12 = 0.8 * np.exp(1j * freq * 2 * np.pi)

fig, axs = plt.subplots(2, 1, figsize=(8, 6))
axs[0].plot(freq, 20 * np.log10(np.abs(s11)), label='S11')
axs[0].set_title('Subplot 1')
axs[1].plot(freq, 20 * np.log10(np.abs(s12)), label='S12')
axs[1].set_title('Subplot 2')
fig.suptitle("Ideal 50-Ohm balun", pad=0)
plt.savefig("./test/test_rf/test_simple_pad0.png")
plt.clf()
print("Saved test_simple_pad0.png (pad=0)")

fig, axs = plt.subplots(2, 1, figsize=(8, 6))
axs[0].plot(freq, 20 * np.log10(np.abs(s11)), label='S11')
axs[0].set_title('Subplot 1')
axs[1].plot(freq, 20 * np.log10(np.abs(s12)), label='S12')
axs[1].set_title('Subplot 2')
fig.suptitle("Ideal 50-Ohm balun", pad=-5)
plt.savefig("./test/test_rf/test_simple_pad_neg5.png")
plt.clf()
print("Saved test_simple_pad_neg5.png (pad=-5)")

fig, axs = plt.subplots(2, 1, figsize=(8, 6))
axs[0].plot(freq, 20 * np.log10(np.abs(s11)), label='S11')
axs[0].set_title('Subplot 1')
axs[1].plot(freq, 20 * np.log10(np.abs(s12)), label='S12')
axs[1].set_title('Subplot 2')
fig.suptitle("Ideal 50-Ohm balun", pad=5)
plt.savefig("./test/test_rf/test_simple_pad5.png")
plt.clf()
print("Saved test_simple_pad5.png (pad=5)")

print("\nDone!")
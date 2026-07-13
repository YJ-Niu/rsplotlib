import os
import rsplotlib.pyplot as plt

OUT = os.path.join(os.path.dirname(__file__), '_font_probe.png')

plt.rcParams['figure.figsize'] = [6, 4]
plt.rcParams['font.size'] = 30  # 3x of 10
fig = plt.figure(dpi=120)
ax = plt.gca()
ax.set_facecolor('#E5E5E5')
fig.set_facecolor('white')
ax.tick_params(color='#555555', labelcolor='#555555', labelsize=27)  # 3x of 9
plt.grid(True, color='white', linestyle='-')

ax.plot([0, 1, 2, 3], [1, 3, 2, 5], label='S11')
ax.plot([0, 1, 2, 3], [2, 1, 4, 3], label='S21')
ax.set_xlabel('Frequency (GHz)')
ax.set_ylabel('Magnitude (dB)')
ax.legend()

plt.savefig(OUT)
print("saved", OUT)

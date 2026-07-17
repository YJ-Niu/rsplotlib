import skrf as rf
from skrf.circuit import Circuit
import rsnumpy as np

rf.stylely()

Z_0 = 50
Z_L = 75
theta = 0

freq = rf.Frequency(start=1, stop=2, unit='GHz', npoints=3)

tline_media = rf.media.DefinedGammaZ0(freq, z0=Z_0)
delay_load = tline_media.delay_load(rf.tlineFunctions.zl_2_Gamma0(Z_0, Z_L), theta, unit='deg', name='delay_load')

port1 = Circuit.Port(freq, 'port1', z0=Z_0)

cnx = [
    [(port1, 0), (delay_load, 0)]
]
cir = Circuit(cnx)

print("=== Circuit Debug Info ===")
print("dim:", cir.dim)
print("port_indexes:", cir.port_indexes)
print("connections:", cir.connections)
print("X_F shape:", cir.X_F.shape)
print("T shape:", np.array(cir.T).shape)

# Reproduce the s_external calculation
port_indexes = cir.port_indexes
in_idxs = [(i,) for i in range(cir.dim) if i not in port_indexes]
ext_idxs = [(i,) for i in port_indexes]
ext_l, in_l = len(ext_idxs), len(in_idxs)

print("\nin_idxs:", in_idxs)
print("ext_idxs:", ext_idxs)
print("ext_l:", ext_l)
print("in_l:", in_l)

# generate index slices for each sub-matrices
idx_a, idx_b, idx_c, idx_d = (
    np.repeat(i, l, axis=1)
    for i, l in (
        (ext_idxs, ext_l),
        (ext_idxs, in_l),
        (in_idxs, ext_l),
        (in_idxs, in_l),
    )
)

print("\nidx_a:", idx_a)
print("idx_b:", idx_b)
print("idx_c:", idx_c)
print("idx_d:", idx_d)

# sub-matrices index, Matrix = [[A, B], [C, D]]]
A_idx = (slice(None), idx_a, idx_a.T)
B_idx = (slice(None), idx_b, idx_c.T)
C_idx = (slice(None), idx_c, idx_b.T)
D_idx = (slice(None), idx_d, idx_d.T)

print("\nA_idx:", A_idx)
print("B_idx:", B_idx)
print("C_idx:", C_idx)
print("D_idx:", D_idx)

# Get the buffer of global matrix in f-order [X_T] and intermediate temporary matrix [T]
# [T] = - [C] @ [X]
x, t = cir.X_F, np.array(cir.T)
print("\nx shape:", x.shape)
print("t shape:", t.shape)

print("\nBefore einsum:")
print("t[0]:", t[0])

np.einsum('...ii->...i', t)[:] += 1

print("\nAfter einsum:")
print("t[0]:", t[0])

# Get the sub-matrices
print("\nt[D_idx] shape:", t[D_idx].shape)
print("t[C_idx] shape:", t[C_idx].shape)
print("t[A_idx] shape:", t[A_idx].shape)
print("t[B_idx] shape:", t[B_idx].shape)

print("\nt[D_idx][0]:", t[D_idx][0])
print("t[C_idx][0]:", t[C_idx][0])
print("t[A_idx][0]:", t[A_idx][0])
print("t[B_idx][0]:", t[B_idx][0])

print("\nx[A_idx][0]:", x[A_idx][0])
print("x[B_idx][0]:", x[B_idx][0])

# Try solve
try:
    tmp_mat = np.linalg.solve(t[D_idx], t[C_idx])
    print("\n solve succeeded:")
    print("tmp_mat[0]:", tmp_mat[0])
except np.linalg.LinAlgError as e:
    print("\n solve failed:", e)
    tmp_mat = np.zeros((freq.npoints, len(in_idxs), len(ext_idxs)), dtype='complex')
    for i in range(freq.npoints):
        tmp_mat[i, :, :] = np.linalg.lstsq(
            t[i, D_idx[1], D_idx[2]], t[i, C_idx[1], C_idx[2]], rcond=None)[0]
    print("tmp_mat[0]:", tmp_mat[0])

# Calculate S_ext
S_ext = (x[A_idx] - x[B_idx] @ tmp_mat) @ np.linalg.inv(
    t[A_idx] - t[B_idx] @ tmp_mat
)

print("\nS_ext[0]:", S_ext[0])

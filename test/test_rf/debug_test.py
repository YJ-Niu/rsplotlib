import rsnumpy as np

# Test the reflection coefficient calculation
# Z_0 = 50, Z_L = 75
# Expected: Gamma = (75-50)/(75+50) = 0.2

# Create a simple test case
# Simulate what circuit.py does

# Test solve with a simple complex matrix
A = [[1+0j, 0], [0, 1+0j]]
b = [[0.2], [0]]

print("Testing solve with simple matrix:")
print("A =", A)
print("b =", b)

try:
    x = np.linalg.solve(A, b)
    print("solve result:", x)
except Exception as e:
    print("solve error:", e)

# Test inv
print("\nTesting inv:")
try:
    A_inv = np.linalg.inv(A)
    print("inv result:", A_inv)
except Exception as e:
    print("inv error:", e)

# Test matmul
print("\nTesting matmul:")
try:
    result = np.linalg.matmul(A, [[1], [0]])
    print("matmul result:", result)
except Exception as e:
    print("matmul error:", e)

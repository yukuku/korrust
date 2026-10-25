package declarations.probes
import declarations.support.Trace
fun localFunctions(seed: Int): Int { var calls = 0; fun factorial(n: Int): Int { calls += 1; if (n <= 1) return seed; return n * factorial(n - 1) }; val result = factorial(4); return result + calls }

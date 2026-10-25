package declarations.probes
import declarations.support.JavaBase
import declarations.support.Trace
fun tryValue(value: Int): Int = try { if (value < 0) throw IllegalArgumentException("negative"); 20 / value } catch (e: ArithmeticException) { 7 } catch (e: IllegalArgumentException) { 8 } finally { Trace.mark(1) }
fun finallyReturn(): Int { try { return Trace.mark(2) } finally { Trace.mark(3) } }
fun finallyOverride(): Int { try { return 2 } finally { return 9 } }
fun finallyThrow() { try { throw IllegalStateException("original") } finally { Trace.mark(4) } }
fun finallyBypass(): Int { try { try { return 1 } catch(error: IllegalArgumentException) { Trace.mark(9); return 2 } finally { throw IllegalArgumentException("x") } } catch(error: IllegalArgumentException) { return 8 } }
fun nestedCatchFinally(): Int { try { return 5 } finally { try { val zero = 0; Trace.mark(10 / zero) } catch(error: ArithmeticException) { Trace.mark(3) } } }
fun rethrowSame(original: RuntimeException): Int { try { throw original } catch(error: RuntimeException) { throw error } finally { Trace.mark(8) } }
class TryJoinChild(value: Int): JavaBase(value)
fun tryReference(fallback: Boolean): JavaBase = try { if(fallback) throw IllegalArgumentException("fallback"); TryJoinChild(4) } catch(error: IllegalArgumentException) { JavaBase(8) }
fun mutableReference(fallback: Boolean): JavaBase { var result: JavaBase = TryJoinChild(4); if(fallback) result = JavaBase(8); return result }

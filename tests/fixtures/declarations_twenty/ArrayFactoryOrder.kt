package declarations.boundary
import declarations.support.Trace
import java.lang.NegativeArraySizeException
fun initializer(): (Int) -> String { Trace.mark(9); return { index -> Trace.mark(index + 1); "v$index" } }
fun negativeArray(): Int = try { Array<String>(-1, initializer()); 0 } catch(error: NegativeArraySizeException) { 1 }
fun zeroArray(): Int { Array<String>(0, initializer()); return 2 }

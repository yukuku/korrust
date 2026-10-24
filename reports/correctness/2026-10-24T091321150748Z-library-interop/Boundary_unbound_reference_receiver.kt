import interop.lib.Counter
import interop.lib.BaseSource
fun bad(): Int { val block: (BaseSource, Int) -> Int = Counter::add; return 1 }

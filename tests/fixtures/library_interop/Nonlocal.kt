package interop.probes
import interop.lib.*
import interop.dep.JavaApi

fun nonlocalProbe(): Int { gate { return 37 }; return -1 }
fun nonlocalLongProbe(): Long { gate { return 1234567890123L }; return -1L }
fun nonlocalStringProbe(): String { gate { return "done" }; return "bad" }
fun nonlocalPrefixProbe(): Int = 1 + gate { return 37 }
fun nonlocalBranchProbe(flag: Boolean): Int { branch(flag) { if (it == 1) return 37; it }; return -1 }
fun conditionalNonlocalProbe(flag: Boolean): Long { val captured = 5L; gate { if (flag) return captured; 7 }; return 9L }
fun nestedNonlocalProbe(): Long { gate { gate { return 39L }; 0 }; return -1L }

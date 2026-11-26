import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import kotlinx.coroutines.Dispatchers
import targets.checkpoint
fun local(flag: Boolean): String = run { if (flag) return@run "early"; "late" }
fun outer(flag: Boolean): Int { val action: () -> Int = outer@{ run { if (flag) return@outer 7 }; 9 }; return action() }
fun middle(): Int = run outer@{ val got = run middle@{ run { return@middle 3 }; 99 }; got + 10 }
fun unwind(): Int { val action: () -> Int = out@{ Probe().use { Probe.mark("B"); return@out 7 }; 99 }; return action() }
fun partial(): Int { val action: () -> Int = out@{ Probe().use { val got = run mid@{ Probe().use { return@mid 3 }; 90 }; Probe.mark("A"); got + 1 } }; return action() }
fun shadow(): Int = run tag@{ val nested = run tag@{ run { return@tag 4 }; 2 }; nested + 1 }



fun resumed(flag: Boolean): Int = runBlocking {
    withContext(Dispatchers.Default) outer@{
        val value = checkpoint()
        run { if (flag) return@outer value + 2 }
        value + 4
    }
}
fun resumedUnit(): Int {
    Probe.trace = ""
    runBlocking {
        withContext(Dispatchers.Default) outer@{
            checkpoint()
            run { Probe.mark("U"); return@outer }
            Probe.mark("X")
        }
    }
    return Probe.trace.length
}

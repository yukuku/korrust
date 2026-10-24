package twenty
import twenty.dep.Trace
class CompanionInit(val value: Int) {
    companion object {
        val FIRST: Int = Trace.mark("X", 4)
        val SECOND: Int = Trace.mark("Y", FIRST + 2)
        val INSTANCE: CompanionInit = CompanionInit(SECOND)
    }
}

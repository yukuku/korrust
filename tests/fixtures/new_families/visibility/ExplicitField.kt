package visibilityexplicit
import visibilityprobe.FakeCompanion
import visibilityprobe.SafeTarget
fun write(value: Int): Int { FakeCompanion.Companion.value = value; return FakeCompanion.Companion.value }

import anonymouslibrary.*
import receiverlibrary.checkpoint
fun use(): UnitSink = object : UnitSink { override suspend fun run(value: Unit): Int = 1 }
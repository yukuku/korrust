import anonymouslibrary.*
import receiverlibrary.checkpoint
fun use(): NullableUnit = object : NullableUnit { override suspend fun run(): Unit? = null }
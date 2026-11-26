import anonymouslibrary.*
import receiverlibrary.checkpoint
fun use(): GenericMethod = object : GenericMethod { override suspend fun <T> run(value: T): T = value }
package nocompatconsumer
import nocompat.Api
class Direct: Api
fun value(): String = Direct().echo("ok")

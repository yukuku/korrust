suspend fun original(): String = "x"
fun bounded(): suspend () -> String = ::original
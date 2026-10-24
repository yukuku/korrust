import interop.dep.JavaApi
fun bad(): Int = JavaApi.bounded<Int>(1)

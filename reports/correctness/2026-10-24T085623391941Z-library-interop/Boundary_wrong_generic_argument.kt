import interop.dep.JavaApi
fun bad(): String = JavaApi.identity<String>(1)

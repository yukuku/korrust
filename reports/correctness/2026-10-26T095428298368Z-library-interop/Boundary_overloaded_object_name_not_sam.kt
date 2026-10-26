import interop.dep.JavaApi
fun bad(): Int = JavaApi.requireNotSam { }

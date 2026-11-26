import library.*
fun classifier(): Class<*> = classOf<List<String>>()
fun isSet(scope: Scope, type: Class<*>): Boolean = scope.same<Set<Int>>(type)

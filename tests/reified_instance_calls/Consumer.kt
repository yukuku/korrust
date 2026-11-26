package fixture
fun text(scope: Scope, input: Any): Boolean = scope.accepts<String>(input)
fun transform(scope: Scope, input: Any): String? = scope.checked<String>(input) { it + "!" }

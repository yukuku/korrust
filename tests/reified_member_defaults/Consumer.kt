import matcher.Scope
fun defaulted(scope: Scope): Boolean = scope.check("text")
fun named(scope: Scope): Boolean = scope.check("text", invert = true)
fun explicit(scope: Scope): Boolean = scope.check("text", number = 8L)

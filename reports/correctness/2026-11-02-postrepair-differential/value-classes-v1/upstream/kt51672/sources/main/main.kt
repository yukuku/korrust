interface C : B

fun box(): String {
    return object : C {}.f(S("OK")).value
}

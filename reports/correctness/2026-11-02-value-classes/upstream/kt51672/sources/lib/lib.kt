@JvmInline
value class S(val value: String)

interface A {
    fun f(s: S): S = s
}

interface B : A


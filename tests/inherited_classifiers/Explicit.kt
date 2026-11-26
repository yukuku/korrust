package inherited.explicit
import inherited.library.JavaTypes.Base
import inherited.library.JavaTypes.Other.Result
class Derived: Base() { fun identity(value: Result): Result = value }

class GenericDerived<Result>: Base() {
    fun constructed(): Any = Result("generic-import")
    fun staticName(): String = Result.make("generic-static").tag()
    fun identity(value: Result): Result = value
}

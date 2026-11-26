package constructorvarargs
class InvalidCollision(vararg val values: String) {
    constructor(values: Array<String>) : this(*values)
}

fun box(): String {
    require(isinstance<String>("abc"))
    require(isinstance<Int>(1))
    require(!isinstance<Int>("abc"))

    return "OK"
}

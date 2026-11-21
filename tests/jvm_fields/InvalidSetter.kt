class Bad {
    @JvmField var value: String = "setter"
        set(input) { field = input }
}

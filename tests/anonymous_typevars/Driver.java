package typevars;
import java.lang.reflect.*;
import java.util.*;
public class Driver {
    static void check(boolean condition) { if (!condition) throw new AssertionError(); }
    static TypeVariable<?> variable(Type type, GenericDeclaration owner, String name) {
        if (type instanceof WildcardType) type = ((WildcardType) type).getUpperBounds()[0];
        check(type instanceof TypeVariable<?>);
        TypeVariable<?> result = (TypeVariable<?>) type;
        check(result.getName().equals(name));
        check(result.getGenericDeclaration().equals(owner));
        check(result.equals(Arrays.stream(owner.getTypeParameters()).filter(v -> v.getName().equals(name)).findFirst().orElseThrow()));
        return result;
    }
    static void resolved(Type type, Set<GenericDeclaration> allowed) {
        check(type != null);
        if (type instanceof TypeVariable<?>) {
            TypeVariable<?> variable = (TypeVariable<?>) type;
            check(allowed.contains(variable.getGenericDeclaration()));
            check(Arrays.asList(variable.getGenericDeclaration().getTypeParameters()).contains(variable));
        } else if (type instanceof ParameterizedType) {
            for (Type argument : ((ParameterizedType) type).getActualTypeArguments()) resolved(argument, allowed);
        } else if (type instanceof GenericArrayType) {
            resolved(((GenericArrayType) type).getGenericComponentType(), allowed);
        } else if (type instanceof WildcardType) {
            for (Type bound : ((WildcardType) type).getUpperBounds()) resolved(bound, allowed);
            for (Type bound : ((WildcardType) type).getLowerBounds()) resolved(bound, allowed);
        }
    }
    static void anonymous(Object value, Class<?> parent, GenericDeclaration declaration, String name, Method enclosing, boolean nested, String methodName) throws Exception {
        Class<?> type = value.getClass();
        check(type.isAnonymousClass() && !type.isLocalClass() && !type.isMemberClass());
        check(type.getTypeParameters().length == 0);
        check(type.getEnclosingClass().equals(enclosing.getDeclaringClass()));
        check(type.getEnclosingMethod().equals(enclosing));
        Type generic = parent.isInterface() ? type.getGenericInterfaces()[0] : type.getGenericSuperclass();
        check(generic instanceof ParameterizedType);
        ParameterizedType supertype = (ParameterizedType) generic;
        check(supertype.getRawType().equals(parent));
        Type parameter = supertype.getActualTypeArguments()[0];
        if (nested) {
            check(parameter instanceof ParameterizedType);
            check(((ParameterizedType) parameter).getRawType().equals(List.class));
            parameter = ((ParameterizedType) parameter).getActualTypeArguments()[0];
            check(parameter instanceof WildcardType);
        }
        variable(parameter, declaration, name);
        TypeVariable<?> declared = Arrays.stream(declaration.getTypeParameters()).filter(v -> v.getName().equals(name)).findFirst().orElseThrow();
        Class<?> erased = (Class<?>) declared.getBounds()[0];
        Method implementation = type.getDeclaredMethod(methodName, nested ? List.class : erased);
        Type input = implementation.getGenericParameterTypes()[0];
        Type output = implementation.getGenericReturnType();
        if (nested) {
            input = ((ParameterizedType) input).getActualTypeArguments()[0];
            output = ((ParameterizedType) output).getActualTypeArguments()[0];
        }
        variable(input, declaration, name);
        variable(output, declaration, name);
        Set<GenericDeclaration> allowed = new HashSet<>(Arrays.asList(declaration, enclosing.getDeclaringClass()));
        for (Field field : type.getDeclaredFields()) {
            Type captured = field.getGenericType();
            resolved(captured, allowed);
            if (captured instanceof TypeVariable<?>) variable(captured, declaration, name);
        }
        for (Constructor<?> constructor : type.getDeclaredConstructors()) {
            check(constructor.getGenericParameterTypes().length == constructor.getParameterCount());
            for (Type captured : constructor.getGenericParameterTypes()) {
                resolved(captured, allowed);
                if (captured instanceof TypeVariable<?>) variable(captured, declaration, name);
            }
        }
    }
    public static void main(String[] args) throws Exception {
        Base<String> fromFunction = ModelKt.fromFunction("captured");
        check(fromFunction.getSeed().equals("captured") && fromFunction.apply("ignored").equals("captured"));
        Method function = ModelKt.class.getDeclaredMethod("fromFunction", Object.class);
        anonymous(fromFunction, Base.class, function, "T", function, false, "apply");
        List<String> list = Collections.singletonList("item");
        Mapper<List<String>> nested = ModelKt.fromNested(list);
        check(nested.map(Collections.emptyList()) == list);
        Method nestedFunction = ModelKt.class.getDeclaredMethod("fromNested", List.class);
        anonymous(nested, Mapper.class, nestedFunction, "T", nestedFunction, true, "map");
        Holder<String> holder = new Holder<>("outer");
        Base<String> fromClass = holder.fromClass();
        check(fromClass.getSeed().equals("outer") && fromClass.apply("ignored").equals("outer"));
        Method classMethod = Holder.class.getDeclaredMethod("fromClass");
        anonymous(fromClass, Base.class, Holder.class, "T", classMethod, false, "apply");
        Mapper<Integer> member = holder.fromMember(7);
        check(member.map(3) == 7);
        Method memberMethod = Holder.class.getDeclaredMethod("fromMember", Object.class);
        anonymous(member, Mapper.class, memberMethod, "U", memberMethod, false, "map");
        Mapper<String> initialized = holder.getInitialized();
        check(initialized.map("ignored").equals("outer"));
        Class<?> initializedType = initialized.getClass();
        check(initializedType.isAnonymousClass());
        check(initializedType.getEnclosingClass().equals(Holder.class));
        check(initializedType.getEnclosingMethod() == null);
        check(initializedType.getEnclosingConstructor().equals(Holder.class.getDeclaredConstructor(Object.class)));
        Type t = ((ParameterizedType) initializedType.getGenericInterfaces()[0]).getActualTypeArguments()[0];
        variable(t, Holder.class, "T");
        Method map = initializedType.getDeclaredMethod("map", Object.class);
        variable(map.getGenericParameterTypes()[0], Holder.class, "T");
        variable(map.getGenericReturnType(), Holder.class, "T");
        Base<Integer> bounded = ModelKt.bounded(13);
        check(bounded.getSeed() == 13 && bounded.apply(2) == 13);
        Method boundedMethod = ModelKt.class.getDeclaredMethod("bounded", Number.class);
        anonymous(bounded, Base.class, boundedMethod, "T", boundedMethod, false, "apply");
        Mapper<String> fromInterface = new FactoryImpl().make("interface");
        check(fromInterface.map("ignored").equals("interface"));
        Method interfaceMethod = Factory.class.getDeclaredMethod("make", Object.class);
        anonymous(fromInterface, Mapper.class, Factory.class, "T", interfaceMethod, false, "map");
        System.out.println("OK");
    }
}

#include <iostream>
using namespace std;

enum TestEnum2 {
    val1 = 1,
    val2
};

enum {
    val1 = 1,
    val2
} TestEnum;

int b = 0;

// comment
String cat = "cat";

struct asd {};

namespace internal {
    int a = 0;
    
    template <typename T> class Array {
        private:
            T* ptr;
            int size;
         
        public:
            Array(T arr[], int s);
            void print();
    };
}
 
template <typename T> Array<T>::Array(T arr[], int s)
{
    ptr = new T[s];
    size = s;
    for (int i = 0; i < size; i++)
        ptr[i] = arr[i];
}

void print() {
}

template <typename T> void asd<T>::Array<T>::print()
{
    for (int i = 0; i < size; i++)
        cout << " " << *(ptr + i);
    cout << endl;
}

Array<int> as(arr, 5);
Array<int> as = Array<int>(arr, 5);

int main() {
    int arr[5] = { 1, 2, 3, 4, 5 };
    Array<int> a(arr, 5);
    a.print();
    print();
    return 0;
}
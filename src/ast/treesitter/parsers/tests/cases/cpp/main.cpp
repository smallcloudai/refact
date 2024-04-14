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


class Animal {
  public:
    void animalSound() {
      cout << "The animal makes a sound \n";
    }
};

// Derived class
class Pig : public Animal {
  public:
    void animalSound() {
      cout << "The pig says: wee wee \n";
    }
};

// Derived class
class Dog : public Animal {
  public:
    void animalSound() {
      cout << "The dog says: bow wow \n";
    }
};

class GFG_Base {
 
public:
    // virtual function
    virtual void display()
    {
        cout << "Called virtual Base Class function"
             << "\n\n";
    }
 
    void print()
    {
        cout << "Called GFG_Base print function"
             << "\n\n";
    }
};
 
// Declaring a Child Class
class GFG_Child : public GFG_Base {
 
public:
    void display()
    {
        cout << "Called GFG_Child Display Function"
             << "\n\n";
    }
 
    void print()
    {
        cout << "Called GFG_Child print Function"
             << "\n\n";
    }
};



int main() {
    int arr[5] = { 1, 2, 3, 4, 5 };
    Array<int> a(arr, 5);
    a.print();
    print();
    
  Animal myAnimal;
  Pig myPig;
  Dog myDog;

  myAnimal.animalSound();
  myPig.animalSound();
  myDog.animalSound();
  
  GFG_Base* base;

  GFG_Child child;

  base = &child;

  // This will call the virtual function
  base->GFG_Base::display();

  // this will call the non-virtual function
  base->print();
    
    return 0;
}
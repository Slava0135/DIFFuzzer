CXX=g++
CXXFLAGS=-std=c++20 -Wall -Wextra -Werror -pedantic

BUILD=$(CXX) $(CXXFLAGS)
TESTNAME=test

all: executor.o test.o
	${BUILD} executor.o ${TESTNAME}.o -o ${TESTNAME}.out

executor.o: executor.cpp
	${BUILD} -c executor.cpp

test.o: ${TESTNAME}.c
	${BUILD} -c ${TESTNAME}.c

clean:
	rm *.o *.out


/*============================================================================

This C source file is part of the SoftFloat IEEE Floating-Point Arithmetic
Package, Release 3e, by John R. Hauser.

Copyright 2011, 2012, 2013, 2014, 2017 The Regents of the University of
California.  All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

 1. Redistributions of source code must retain the above copyright notice,
    this list of conditions, and the following disclaimer.

 2. Redistributions in binary form must reproduce the above copyright notice,
    this list of conditions, and the following disclaimer in the documentation
    and/or other materials provided with the distribution.

 3. Neither the name of the University nor the names of its contributors may
    be used to endorse or promote products derived from this software without
    specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS "AS IS", AND ANY
EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE, ARE
DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE FOR ANY
DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

=============================================================================*/


static unsigned long long cosmic_sf_private_roundToInt64( cosmic_sf_context *ctx, unsigned long long a, uint_fast8_t roundingMode, bool exact )
{
    uint_fast64_t uiA;
    int_fast16_t exp;
    uint_fast64_t uiZ, lastBitMask, roundBitsMask;

    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    uiA = a;
    exp = expF64UI( uiA );
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    if ( exp <= 0x3FE ) {
        if ( !(uiA & UINT64_C( 0x7FFFFFFFFFFFFFFF )) ) return a;
        if ( exact ) ctx->flags |= cosmic_sf_private_flag_inexact;
        uiZ = uiA & packToF64UI( 1, 0, 0 );
        switch ( roundingMode ) {
         case cosmic_sf_private_round_near_even:
            if ( !fracF64UI( uiA ) ) break;
         case cosmic_sf_private_round_near_maxMag:
            if ( exp == 0x3FE ) uiZ |= packToF64UI( 0, 0x3FF, 0 );
            break;
         case cosmic_sf_private_round_min:
            if ( uiZ ) uiZ = packToF64UI( 1, 0x3FF, 0 );
            break;
         case cosmic_sf_private_round_max:
            if ( !uiZ ) uiZ = packToF64UI( 0, 0x3FF, 0 );
            break;
#ifdef SOFTFLOAT_ROUND_ODD
         case cosmic_sf_private_round_odd:
            uiZ |= packToF64UI( 0, 0x3FF, 0 );
            break;
#endif
        }
        goto uiZ;
    }
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    if ( 0x433 <= exp ) {
        if ( (exp == 0x7FF) && fracF64UI( uiA ) ) {
            uiZ = cosmic_sf_private_propagateNaNF64UI( ctx, uiA, 0 );
            goto uiZ;
        }
        return a;
    }
    /*------------------------------------------------------------------------
    *------------------------------------------------------------------------*/
    uiZ = uiA;
    lastBitMask = (uint_fast64_t) 1<<(0x433 - exp);
    roundBitsMask = lastBitMask - 1;
    if ( roundingMode == cosmic_sf_private_round_near_maxMag ) {
        uiZ += lastBitMask>>1;
    } else if ( roundingMode == cosmic_sf_private_round_near_even ) {
        uiZ += lastBitMask>>1;
        if ( !(uiZ & roundBitsMask) ) uiZ &= ~lastBitMask;
    } else if (
        roundingMode
            == (signF64UI( uiZ ) ? cosmic_sf_private_round_min : cosmic_sf_private_round_max)
    ) {
        uiZ += roundBitsMask;
    }
    uiZ &= ~roundBitsMask;
    if ( uiZ != uiA ) {
#ifdef SOFTFLOAT_ROUND_ODD
        if ( roundingMode == cosmic_sf_private_round_odd ) uiZ |= lastBitMask;
#endif
        if ( exact ) ctx->flags |= cosmic_sf_private_flag_inexact;
    }
 uiZ:
    return uiZ;

}

